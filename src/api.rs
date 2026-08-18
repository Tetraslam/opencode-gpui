use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};

use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::{Client as HttpClient, StatusCode};
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::{runtime::Handle, sync::oneshot};
use url::Url;

use crate::{
    event::{Event, Payload},
    model::{Health, MessageRecord, Session, sort_sessions},
};

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

pub struct EventSubscription {
    receiver: tokio::sync::mpsc::Receiver<Result<Event, String>>,
    cancel: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid OpenCode server URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("OpenCode server URL cannot be used as an HTTP base URL")]
    InvalidBaseUrl,
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

    /// Fetches the newest messages for one session, bounded by `limit`.
    ///
    /// # Errors
    ///
    /// Returns an error when the runtime stops or the server request fails.
    pub async fn messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<MessageRecord>, Error> {
        let inner = Arc::clone(&self.inner);
        let session_id = session_id.to_owned();
        self.inner
            .runtime
            .run(async move { inner.get_messages(&session_id, limit.clamp(1, 1_000)).await })
            .await
    }

    /// Opens a cancellable server-sent event subscription for this client's directory scope.
    ///
    /// # Errors
    ///
    /// Returns an error when the runtime stops or the server rejects the subscription.
    pub async fn subscribe_events(&self) -> Result<EventSubscription, Error> {
        let inner = Arc::clone(&self.inner);
        self.inner
            .runtime
            .run(async move { inner.subscribe_events().await })
            .await
    }
}

impl ClientInner {
    async fn subscribe_events(&self) -> Result<EventSubscription, Error> {
        let mut url = self.url(&["event"])?;
        if let Some(directory) = &self.directory {
            url.query_pairs_mut().append_pair("directory", directory);
        }
        let response = self.request_url(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "response body unavailable".into());
            return Err(Error::Http { status, message });
        }

        let mut stream = response.bytes_stream().eventsource();
        let (sender, receiver) = tokio::sync::mpsc::channel(1_024);
        let (cancel, mut cancelled) = oneshot::channel();
        tokio::spawn(async move {
            loop {
                let item = tokio::select! {
                    _ = &mut cancelled => break,
                    item = stream.next() => item,
                };
                let Some(item) = item else {
                    break;
                };
                let event = item.map_err(|error| error.to_string()).and_then(|event| {
                    serde_json::from_str::<Payload>(&event.data)
                        .map(Payload::into_event)
                        .map_err(|error| error.to_string())
                });
                let failed = event.is_err();
                if sender.send(event).await.is_err() || failed {
                    break;
                }
            }
        });

        Ok(EventSubscription {
            receiver,
            cancel: Some(cancel),
        })
    }

    async fn get_messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<MessageRecord>, Error> {
        let mut url = self.url(&["session", session_id, "message"])?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(directory) = &self.directory {
                query.append_pair("directory", directory);
            }
            query.append_pair("limit", &limit.to_string());
        }
        Self::send_json(self.request_url(url)).await
    }

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
        Ok(self.request_url(url))
    }

    fn request_url(&self, url: Url) -> reqwest::RequestBuilder {
        let request = self.http.get(url);
        match (&self.username, &self.password) {
            (Some(username), Some(password)) => request.basic_auth(username, Some(password)),
            _ => request,
        }
    }

    fn url(&self, segments: &[&str]) -> Result<Url, Error> {
        let mut url = self.base_url.clone();
        let mut path = url
            .path_segments_mut()
            .map_err(|()| Error::InvalidBaseUrl)?;
        path.pop_if_empty();
        path.extend(segments);
        drop(path);
        Ok(url)
    }

    async fn send_json<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<T, Error> {
        let response = request
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await?;
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

impl EventSubscription {
    pub async fn next(&mut self) -> Option<Result<Event, String>> {
        self.receiver.recv().await
    }

    pub fn try_next(&mut self) -> Option<Result<Event, String>> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for EventSubscription {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
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
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 2048];
                let length = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..length]);
                let body = if request.starts_with("GET /global/health ") {
                    r#"{"healthy":true,"version":"1.18.16"}"#
                } else if request.starts_with("GET /session?directory=%2Fworkspace ") {
                    r#"[{"id":"ses_1","projectID":"prj_1","directory":"/workspace","title":"first","version":"1.18.16","time":{"created":1,"updated":2}},{"id":"ses_2","projectID":"prj_1","directory":"/workspace","title":"second","version":"1.18.16","time":{"created":2,"updated":3}}]"#
                } else if request
                    .starts_with("GET /session/ses_2/message?directory=%2Fworkspace&limit=100 ")
                {
                    r#"[{"info":{"id":"msg_1","sessionID":"ses_2","role":"user","time":{"created":3},"agent":"build","model":{"providerID":"openai","modelID":"gpt-test"}},"parts":[{"id":"part_1","sessionID":"ses_2","messageID":"msg_1","type":"text","text":"hello"}]}]"#
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
        let messages = pollster::block_on(client.messages("ses_2", 100)).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].info.role(), "you");
        assert_eq!(messages[0].parts[0].text(), Some("hello"));
        server.join().unwrap();
    }

    #[test]
    fn streams_directory_scoped_events() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let length = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..length]);
            assert!(request.starts_with("GET /event?directory=%2Fworkspace "));
            let body = "data: {\"type\":\"server.connected\",\"properties\":{}}\n\ndata: {\"type\":\"session.status\",\"properties\":{\"sessionID\":\"ses_1\",\"status\":{\"type\":\"busy\"}}}\n\n";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let client = Client::new(
            &format!("http://{address}"),
            Some("/workspace".into()),
            None,
            None,
        )
        .unwrap();

        pollster::block_on(async {
            let mut events = client.subscribe_events().await.unwrap();
            assert!(matches!(
                events.next().await.unwrap().unwrap(),
                Event::ServerConnected
            ));
            assert!(matches!(
                events.next().await.unwrap().unwrap(),
                Event::SessionStatus { session_id, .. } if session_id == "ses_1"
            ));
        });
        server.join().unwrap();
    }
}
