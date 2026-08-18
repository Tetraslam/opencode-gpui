use std::{env, sync::Arc, time::SystemTime};

use gpui::{
    App, Application, Bounds, Context, SharedString, Task, TitlebarOptions, Window, WindowBounds,
    WindowOptions, div, prelude::*, px, rgb, size, uniform_list,
};
use opencode_gpui::{
    api::{Bootstrap, Client},
    model::Session,
};

const BACKGROUND: u32 = 0x000b_0d10;
const SURFACE: u32 = 0x0012_151a;
const BORDER: u32 = 0x0025_2a31;
const TEXT: u32 = 0x00e7_e9ed;
const MUTED: u32 = 0x0089_919d;
const ACCENT: u32 = 0x007d_d3a7;

enum LoadState {
    Loading,
    Ready {
        version: SharedString,
        sessions: Arc<[Session]>,
    },
    Failed(SharedString),
}

struct Workspace {
    server: SharedString,
    state: LoadState,
    _load: Task<()>,
}

impl Workspace {
    fn new(cx: &mut Context<Self>) -> Self {
        let server =
            env::var("OPENCODE_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:4096".into());
        let directory = env::var("OPENCODE_DIRECTORY").ok();
        let username = env::var("OPENCODE_SERVER_USERNAME").ok();
        let password = env::var("OPENCODE_SERVER_PASSWORD").ok();
        let client = Client::new(&server, directory, username, password);
        let load = cx.spawn(async move |workspace, cx| {
            let result = match client {
                Ok(client) => client.bootstrap().await,
                Err(error) => Err(error),
            };
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.apply_bootstrap(result);
                cx.notify();
            });
        });

        Self {
            server: server.into(),
            state: LoadState::Loading,
            _load: load,
        }
    }

    fn apply_bootstrap(&mut self, result: Result<Bootstrap, opencode_gpui::api::Error>) {
        self.state = match result {
            Ok(bootstrap) => LoadState::Ready {
                version: bootstrap.health.version.into(),
                sessions: bootstrap.sessions.into(),
            },
            Err(error) => LoadState::Failed(error.to_string().into()),
        };
    }

    fn render_session(session: &Session) -> gpui::AnyElement {
        let title: SharedString = if session.title.trim().is_empty() {
            "Untitled session".into()
        } else {
            session.title.clone().into()
        };
        let directory: SharedString = session.directory.clone().into();
        let age: SharedString = relative_time(session.time.updated).into();

        div()
            .id(SharedString::from(session.id.clone()))
            .h(px(66.0))
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(rgb(BORDER))
            .hover(|row| row.bg(rgb(0x0019_1d23)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .child(div().text_color(rgb(TEXT)).child(title))
                            .child(
                                div()
                                    .mt_1()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .overflow_hidden()
                                    .child(directory),
                            ),
                    )
                    .child(div().ml_4().text_sm().text_color(rgb(MUTED)).child(age)),
            )
            .into_any_element()
    }
}

impl Render for Workspace {
    #[allow(clippy::too_many_lines)]
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match &self.state {
            LoadState::Loading => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(MUTED))
                .child("Connecting to OpenCode...")
                .into_any_element(),
            LoadState::Failed(error) => div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .child(div().text_color(rgb(TEXT)).child("OpenCode is unavailable"))
                .child(div().text_sm().text_color(rgb(MUTED)).child(error.clone()))
                .child(
                    div()
                        .mt_2()
                        .text_sm()
                        .text_color(rgb(MUTED))
                        .child("Start it with: opencode serve --port 4096"),
                )
                .into_any_element(),
            LoadState::Ready { sessions, .. } => {
                let sessions = Arc::clone(sessions);
                uniform_list(
                    "sessions",
                    sessions.len(),
                    cx.processor(
                        move |_workspace, range: std::ops::Range<usize>, _window, _cx| {
                            range
                                .map(|index| Self::render_session(&sessions[index]))
                                .collect()
                        },
                    ),
                )
                .h_full()
                .into_any_element()
            }
        };

        let (status, count): (SharedString, Option<usize>) = match &self.state {
            LoadState::Loading => ("connecting".into(), None),
            LoadState::Failed(_) => ("offline".into(), None),
            LoadState::Ready { version, sessions } => {
                (format!("opencode {version}").into(), Some(sessions.len()))
            }
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(BACKGROUND))
            .font_family("Inter")
            .child(
                div()
                    .h(px(58.0))
                    .flex_none()
                    .px_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(rgb(SURFACE))
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(div().text_lg().text_color(rgb(TEXT)).child("opencode"))
                            .child(div().text_sm().text_color(rgb(MUTED)).child("sessions")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .when_some(count, |header, count| {
                                header.child(format!("{count} sessions"))
                            })
                            .child(div().size_2().rounded_full().bg(rgb(ACCENT)))
                            .child(status),
                    ),
            )
            .child(content)
            .child(
                div()
                    .h(px(30.0))
                    .flex_none()
                    .px_4()
                    .flex()
                    .items_center()
                    .bg(rgb(SURFACE))
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child(self.server.clone()),
            )
    }
}

fn relative_time(timestamp_ms: u64) -> String {
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        });
    let seconds = now_ms.saturating_sub(timestamp_ms) / 1_000;
    match seconds {
        0..=59 => "now".into(),
        60..=3_599 => format!("{}m", seconds / 60),
        3_600..=86_399 => format!("{}h", seconds / 3_600),
        86_400..=2_591_999 => format!("{}d", seconds / 86_400),
        _ => format!("{}mo", seconds / 2_592_000),
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1040.0), px(760.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("OpenCode".into()),
                    ..Default::default()
                }),
                app_id: Some("ai.opencode.gpui".into()),
                ..Default::default()
            },
            |_, cx| cx.new(Workspace::new),
        )
        .expect("failed to open the OpenCode window");
        cx.activate(true);
    });
}
