use std::time::Duration;

use gpui::{AppContext, Context, Timer};
use opencode_gpui::api::{Bootstrap, Client, Error};

use super::{Workspace, local_server::ManagedServer};

pub(super) struct PreparedConnection {
    pub(super) url: String,
    pub(super) client: Result<Client, Error>,
    pub(super) server_start: Option<gpui::Task<Result<Option<ManagedServer>, String>>>,
}

pub(super) fn prepare(cx: &Context<Workspace>) -> PreparedConnection {
    let explicit = std::env::var("OPENCODE_SERVER_URL").ok();
    let url = explicit
        .clone()
        .unwrap_or_else(|| super::local_server::DEFAULT_URL.into());
    let client = Client::new(
        &url,
        None,
        std::env::var("OPENCODE_SERVER_USERNAME").ok(),
        std::env::var("OPENCODE_SERVER_PASSWORD").ok(),
    );
    let server_start = super::local_server::autostart_enabled(
        explicit.is_some(),
        std::env::var("OPENCODE_SERVER_AUTOSTART").ok().as_deref(),
    )
    .then(|| cx.background_spawn(async { super::local_server::ensure_running() }));
    PreparedConnection {
        url,
        client,
        server_start,
    }
}

pub(super) struct StartupResult {
    pub(super) bootstrap: Result<Bootstrap, String>,
    pub(super) server: Option<ManagedServer>,
}

pub(super) async fn connect(
    client: Option<Client>,
    setup_error: Option<String>,
    server_start: Option<gpui::Task<Result<Option<ManagedServer>, String>>>,
) -> StartupResult {
    let Some(client) = client else {
        return StartupResult {
            bootstrap: Err(setup_error.unwrap_or_else(|| "client setup failed".into())),
            server: None,
        };
    };
    let autostart = server_start.is_some();
    let initial = bootstrap_attempt(&client).await;
    let mut server = match server_start {
        Some(start) => match start.await {
            Ok(server) => server,
            Err(error) if initial.is_err() => {
                return StartupResult {
                    bootstrap: Err(error),
                    server: None,
                };
            }
            Err(_) => None,
        },
        None => None,
    };
    if initial.is_ok() {
        return StartupResult {
            bootstrap: initial,
            server,
        };
    }
    let mut last_error = initial.expect_err("checked above");
    for delay in [50, 100, 200, 400, 800, 1_600, 3_200] {
        Timer::after(Duration::from_millis(delay)).await;
        if let Some(server) = &mut server {
            match server.exit_status() {
                Ok(Some(status)) => {
                    return StartupResult {
                        bootstrap: Err(format!(
                            "local opencode server exited before becoming ready ({status})"
                        )),
                        server: None,
                    };
                }
                Ok(None) => {}
                Err(error) => {
                    return StartupResult {
                        bootstrap: Err(error),
                        server: None,
                    };
                }
            }
        }
        match bootstrap_attempt(&client).await {
            Ok(bootstrap) => {
                return StartupResult {
                    bootstrap: Ok(bootstrap),
                    server,
                };
            }
            Err(error) => last_error = error,
        }
    }
    if autostart && server.is_none() {
        last_error = format!(
            "port 4096 is occupied but is not responding as OpenCode; free it or set OPENCODE_SERVER_URL ({last_error})"
        );
    }
    StartupResult {
        bootstrap: Err(format!(
            "could not connect to opencode after startup retries: {last_error}"
        )),
        server,
    }
}

async fn bootstrap_attempt(client: &Client) -> Result<Bootstrap, String> {
    let request = async { client.bootstrap().await.map_err(|error| error.to_string()) };
    let timeout = async {
        Timer::after(Duration::from_secs(2)).await;
        Err("OpenCode bootstrap attempt timed out after 2 seconds".into())
    };
    futures_util::pin_mut!(request, timeout);
    match futures_util::future::select(request, timeout).await {
        futures_util::future::Either::Left((result, _))
        | futures_util::future::Either::Right((result, _)) => result,
    }
}
