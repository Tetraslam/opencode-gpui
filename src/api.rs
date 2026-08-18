use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};

use reqwest::{Client as HttpClient, StatusCode};
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::{runtime::Handle, sync::oneshot};
use url::Url;

use crate::model::{Health, Session, sort_sessions};

#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    base_url: Url,
    directory: Option<String>,
    username: Option<String>,
    password: Option<String>,
    http: HttpClient,
    runtime: Runtime,
}

#[derive(Clone)]
struct Runtime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    handle: Handle,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
}

#[derive(Debug)]
pub struct Bootstrap {
    pub health: Health,
    pub sessions: Vec<Session>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid OpenCode server URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("could not start the network runtime")]
    RuntimeStart,
    #[error("the network runtime stopped unexpectedly")]
    RuntimeStopped,
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("OpenCode returned HTTP {status}: {message}")]
    Http { status: StatusCode, message: String },
}

impl Client {
    /// Creates a client for an `OpenCode` server and optional directory scope.
    ///
    /// # Errors
    ///
    /// Returns an error when the URL is invalid, the HTTP client cannot be configured, or the
    /// dedicated network runtime cannot start.
    pub fn new(
        base_url: &str,
        directory: Option<String>,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<Self, Error> {
        let base_url = Url::parse(base_url)?;
        let http = HttpClient::builder()
            .connect_timeout(std::time::Duration::from_secs(3))
            .timeout(std::time::Duration::from_secs(20))
            .build()?;

        Ok(Self {
            inner: Arc::new(ClientInner {
                base_url,
                directory,
                username,
                password,
                http,
                runtime: Runtime::new()?,
            }),
        })
    }

    /// Fetches the server health and session metadata needed for the initial UI state.
    ///
    /// # Errors
    ///
    /// Returns an error when the runtime stops or either server request fails.
    pub async fn bootstrap(&self) -> Result<Bootstrap, Error> {
        let inner = Arc::clone(&self.inner);
        self.inner
            .runtime
            .run(async move {
                let health = inner.get_json::<Health>("global/health").await?;
                let mut sessions = inner.get_sessions().await?;
                sort_sessions(&mut sessions);
                Ok(Bootstrap { health, sessions })
            })
            .await
    }
}

impl ClientInner {
    async fn get_sessions(&self) -> Result<Vec<Session>, Error> {
        let mut request = self.request("session")?;
        if let Some(directory) = &self.directory {
            request = request.query(&[("directory", directory)]);
        }
        Self::send_json(request).await
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        Self::send_json(self.request(path)?).await
    }

    fn request(&self, path: &str) -> Result<reqwest::RequestBuilder, Error> {
        let url = self.base_url.join(path)?;
        let request = self.http.get(url);
        Ok(match (&self.username, &self.password) {
            (Some(username), Some(password)) => request.basic_auth(username, Some(password)),
            _ => request,
        })
    }

    async fn send_json<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<T, Error> {
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "response body unavailable".into());
            return Err(Error::Http { status, message });
        }
        Ok(response.json().await?)
    }
}

impl Runtime {
    fn new() -> Result<Self, Error> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let thread = thread::Builder::new()
            .name("opencode-network".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .thread_name("opencode-http")
                    .enable_all()
                    .build();
                let Ok(runtime) = runtime else {
                    return;
                };
                if ready_tx.send(runtime.handle().clone()).is_err() {
                    return;
                }
                runtime.block_on(async {
                    let _ = shutdown_rx.await;
                });
            })
            .map_err(|_| Error::RuntimeStart)?;
        let handle = ready_rx.recv().map_err(|_| Error::RuntimeStart)?;

        Ok(Self {
            inner: Arc::new(RuntimeInner {
                handle,
                shutdown: Mutex::new(Some(shutdown_tx)),
                thread: Mutex::new(Some(thread)),
            }),
        })
    }

    async fn run<T: Send + 'static>(
        &self,
        future: impl Future<Output = Result<T, Error>> + Send + 'static,
    ) -> Result<T, Error> {
        let (tx, rx) = oneshot::channel();
        self.inner.handle.spawn(async move {
            let _ = tx.send(future.await);
        });
        rx.await.map_err(|_| Error::RuntimeStopped)?
    }
}

impl Drop for RuntimeInner {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.get_mut().ok().and_then(Option::take) {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.get_mut().ok().and_then(Option::take) {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };

    use super::*;

    #[test]
    fn bootstraps_against_the_server_contract() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 2048];
                let length = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..length]);
                let body = if request.starts_with("GET /global/health ") {
                    r#"{"healthy":true,"version":"1.18.16"}"#
                } else if request.starts_with("GET /session?directory=%2Fworkspace ") {
                    r#"[{"id":"ses_1","projectID":"prj_1","directory":"/workspace","title":"first","version":"1.18.16","time":{"created":1,"updated":2}},{"id":"ses_2","projectID":"prj_1","directory":"/workspace","title":"second","version":"1.18.16","time":{"created":2,"updated":3}}]"#
                } else {
                    panic!("unexpected request: {request}");
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });

        let client = Client::new(
            &format!("http://{address}"),
            Some("/workspace".into()),
            None,
            None,
        )
        .unwrap();
        let bootstrap = pollster::block_on(client.bootstrap()).unwrap();

        assert!(bootstrap.health.healthy);
        assert_eq!(bootstrap.health.version, "1.18.16");
        assert_eq!(
            bootstrap
                .sessions
                .iter()
                .map(|session| session.id.as_str())
                .collect::<Vec<_>>(),
            ["ses_2", "ses_1"]
        );
        server.join().unwrap();
    }
}
