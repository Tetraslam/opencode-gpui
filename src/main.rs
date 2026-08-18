use std::{
    collections::HashMap,
    env,
    sync::Arc,
    time::{Duration, SystemTime},
};

use gpui::{
    App, Application, Bounds, ClickEvent, Context, SharedString, Task, Timer, TitlebarOptions,
    Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size, uniform_list,
};
use opencode_gpui::{
    api::{Bootstrap, Client},
    event::{Event, SessionStatus},
    model::{MessageRecord, Part, Session, sort_sessions},
};

const BACKGROUND: u32 = 0x000b_0d10;
const SURFACE: u32 = 0x0012_151a;
const ELEVATED: u32 = 0x0019_1d23;
const SELECTED: u32 = 0x0020_282d;
const BORDER: u32 = 0x0025_2a31;
const TEXT: u32 = 0x00e7_e9ed;
const MUTED: u32 = 0x0089_919d;
const ACCENT: u32 = 0x007d_d3a7;
const WARNING: u32 = 0x00e6_b566;
const DANGER: u32 = 0x00ef_7a7a;
const TIMELINE_LIMIT: usize = 100;

enum ServerState {
    Loading,
    Ready {
        version: SharedString,
        sessions: Arc<Vec<Session>>,
    },
    Failed(SharedString),
}

enum TimelineState {
    Empty,
    Loading {
        session_id: String,
        title: SharedString,
    },
    Ready {
        session_id: String,
        title: SharedString,
        messages: Vec<MessageRecord>,
    },
    Failed {
        session_id: String,
        title: SharedString,
        error: SharedString,
    },
}

impl TimelineState {
    fn session_id(&self) -> Option<&str> {
        match self {
            Self::Empty => None,
            Self::Loading { session_id, .. }
            | Self::Ready { session_id, .. }
            | Self::Failed { session_id, .. } => Some(session_id),
        }
    }

    fn title(&self) -> Option<SharedString> {
        match self {
            Self::Empty => None,
            Self::Loading { title, .. }
            | Self::Ready { title, .. }
            | Self::Failed { title, .. } => Some(title.clone()),
        }
    }
}

struct Workspace {
    client: Option<Client>,
    server: SharedString,
    server_state: ServerState,
    timeline: TimelineState,
    statuses: Arc<HashMap<String, SessionStatus>>,
    live: bool,
    _load: Task<()>,
    _events: Task<()>,
    timeline_load: Option<Task<()>>,
}

impl Workspace {
    fn new(cx: &mut Context<Self>) -> Self {
        let server =
            env::var("OPENCODE_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:4096".into());
        let directory = env::var("OPENCODE_DIRECTORY").ok();
        let username = env::var("OPENCODE_SERVER_USERNAME").ok();
        let password = env::var("OPENCODE_SERVER_PASSWORD").ok();
        let client_result = Client::new(&server, directory, username, password);
        let client = client_result.as_ref().ok().cloned();
        let load_client = client.clone();
        let setup_error = client_result.err().map(|error| error.to_string());
        let load = cx.spawn(async move |workspace, cx| {
            let result = match load_client {
                Some(client) => client.bootstrap().await.map_err(|error| error.to_string()),
                None => Err(setup_error.unwrap_or_else(|| "client setup failed".into())),
            };
            let _ = workspace.update(cx, |workspace, cx| {
                workspace.apply_bootstrap(result);
                cx.notify();
            });
        });
        let events = Self::spawn_event_loop(client.clone(), cx);

        Self {
            client,
            server: server.into(),
            server_state: ServerState::Loading,
            timeline: TimelineState::Empty,
            statuses: Arc::new(HashMap::new()),
            live: false,
            _load: load,
            _events: events,
            timeline_load: None,
        }
    }

    fn spawn_event_loop(client: Option<Client>, cx: &Context<Self>) -> Task<()> {
        cx.spawn(async move |workspace, cx| {
            let Some(client) = client else {
                return;
            };
            let mut retry_delay = Duration::from_millis(250);
            loop {
                let subscription = client.subscribe_events().await;
                let Ok(mut subscription) = subscription else {
                    if workspace
                        .update(cx, |workspace, cx| {
                            workspace.live = false;
                            cx.notify();
                        })
                        .is_err()
                    {
                        return;
                    }
                    Timer::after(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(Duration::from_secs(8));
                    continue;
                };
                retry_delay = Duration::from_millis(250);
                if workspace
                    .update(cx, |workspace, cx| {
                        workspace.live = true;
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }

                let mut disconnected = false;
                while let Some(item) = subscription.next().await {
                    let Ok(first) = item else {
                        break;
                    };
                    let mut batch = vec![first];
                    Timer::after(Duration::from_millis(16)).await;
                    while let Some(item) = subscription.try_next() {
                        if let Ok(event) = item {
                            batch.push(event);
                        } else {
                            disconnected = true;
                            break;
                        }
                    }

                    let rehydrate = batch
                        .iter()
                        .any(|event| matches!(event, Event::ServerConnected));
                    let bootstrap = if rehydrate {
                        client.bootstrap().await.ok()
                    } else {
                        None
                    };
                    if workspace
                        .update(cx, |workspace, cx| {
                            if let Some(bootstrap) = bootstrap {
                                workspace.apply_bootstrap(Ok(bootstrap));
                            }
                            workspace.apply_events(batch);
                            cx.notify();
                        })
                        .is_err()
                    {
                        return;
                    }
                    if disconnected {
                        break;
                    }
                }

                if workspace
                    .update(cx, |workspace, cx| {
                        workspace.live = false;
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
                Timer::after(retry_delay).await;
                retry_delay = (retry_delay * 2).min(Duration::from_secs(8));
            }
        })
    }

    fn apply_bootstrap(&mut self, result: Result<Bootstrap, String>) {
        self.server_state = match result {
            Ok(bootstrap) => ServerState::Ready {
                version: bootstrap.health.version.into(),
                sessions: bootstrap.sessions.into(),
            },
            Err(error) => ServerState::Failed(error.into()),
        };
    }

    fn apply_events(&mut self, events: Vec<Event>) {
        for event in events {
            match event {
                Event::ServerConnected | Event::Unknown(_) => {}
                Event::SessionCreated(session) | Event::SessionUpdated(session) => {
                    self.upsert_session(session);
                }
                Event::SessionDeleted(session) => {
                    if let ServerState::Ready { sessions, .. } = &mut self.server_state {
                        Arc::make_mut(sessions).retain(|current| current.id != session.id);
                    }
                    Arc::make_mut(&mut self.statuses).remove(&session.id);
                    if self.timeline.session_id() == Some(session.id.as_str()) {
                        self.timeline = TimelineState::Empty;
                        self.timeline_load = None;
                    }
                }
                Event::SessionStatus { session_id, status } => {
                    Arc::make_mut(&mut self.statuses).insert(session_id, status);
                }
                Event::SessionIdle { session_id } => {
                    Arc::make_mut(&mut self.statuses).insert(session_id, SessionStatus::Idle);
                }
                Event::MessageUpdated(info) => {
                    if let TimelineState::Ready {
                        session_id,
                        messages,
                        ..
                    } = &mut self.timeline
                        && info.session_id() == session_id
                    {
                        if let Some(message) = messages
                            .iter_mut()
                            .find(|message| message.info.id() == info.id())
                        {
                            message.info = info;
                        } else {
                            messages.push(MessageRecord {
                                info,
                                parts: Vec::new(),
                            });
                        }
                    }
                }
                Event::MessageRemoved {
                    session_id,
                    message_id,
                } => {
                    if let TimelineState::Ready {
                        session_id: selected,
                        messages,
                        ..
                    } = &mut self.timeline
                        && selected == &session_id
                    {
                        messages.retain(|message| message.info.id() != message_id);
                    }
                }
                Event::MessagePartUpdated { part, .. } => {
                    if let TimelineState::Ready {
                        session_id,
                        messages,
                        ..
                    } = &mut self.timeline
                        && part.session_id == *session_id
                        && let Some(message) = messages
                            .iter_mut()
                            .find(|message| message.info.id() == part.message_id)
                    {
                        if let Some(current) = message
                            .parts
                            .iter_mut()
                            .find(|current| current.id == part.id)
                        {
                            *current = part;
                        } else {
                            message.parts.push(part);
                        }
                    }
                }
                Event::MessagePartRemoved {
                    session_id,
                    message_id,
                    part_id,
                } => {
                    if let TimelineState::Ready {
                        session_id: selected,
                        messages,
                        ..
                    } = &mut self.timeline
                        && selected == &session_id
                        && let Some(message) = messages
                            .iter_mut()
                            .find(|message| message.info.id() == message_id)
                    {
                        message.parts.retain(|part| part.id != part_id);
                    }
                }
            }
        }
    }

    fn upsert_session(&mut self, session: Session) {
        let ServerState::Ready { sessions, .. } = &mut self.server_state else {
            return;
        };
        let sessions = Arc::make_mut(sessions);
        if let Some(current) = sessions.iter_mut().find(|current| current.id == session.id) {
            *current = session;
        } else {
            sessions.push(session);
        }
        sort_sessions(sessions);
    }

    fn select_session(&mut self, session_id: String, title: SharedString, cx: &mut Context<Self>) {
        if self.timeline.session_id() == Some(session_id.as_str()) {
            return;
        }
        self.timeline = TimelineState::Loading {
            session_id: session_id.clone(),
            title: title.clone(),
        };
        let Some(client) = self.client.clone() else {
            self.timeline = TimelineState::Failed {
                session_id,
                title,
                error: "OpenCode client is unavailable".into(),
            };
            cx.notify();
            return;
        };

        let requested_id = session_id.clone();
        self.timeline_load = Some(cx.spawn(async move |workspace, cx| {
            let result = client
                .messages(&requested_id, TIMELINE_LIMIT)
                .await
                .map_err(|error| error.to_string());
            let _ = workspace.update(cx, |workspace, cx| {
                if workspace.timeline.session_id() != Some(requested_id.as_str()) {
                    return;
                }
                workspace.timeline = match result {
                    Ok(messages) => TimelineState::Ready {
                        session_id: requested_id,
                        title,
                        messages,
                    },
                    Err(error) => TimelineState::Failed {
                        session_id: requested_id,
                        title,
                        error: error.into(),
                    },
                };
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn render_session(
        session: &Session,
        selected: bool,
        status: Option<&SessionStatus>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let title: SharedString = display_title(session).into();
        let directory: SharedString = session.directory.clone().into();
        let age: SharedString = relative_time(session.time.updated).into();
        let status: Option<SharedString> = match status {
            Some(SessionStatus::Busy) => Some("busy".into()),
            Some(SessionStatus::Retry { attempt, .. }) => Some(format!("retry {attempt}").into()),
            Some(SessionStatus::Idle) | None => None,
        };
        let session_id = session.id.clone();
        let selected_title = title.clone();

        div()
            .id(SharedString::from(session.id.clone()))
            .h(px(66.0))
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(rgb(BORDER))
            .cursor_pointer()
            .when(selected, |row| row.bg(rgb(SELECTED)))
            .hover(|row| row.bg(rgb(ELEVATED)))
            .on_click(
                cx.listener(move |workspace, _event: &ClickEvent, _window, cx| {
                    workspace.select_session(session_id.clone(), selected_title.clone(), cx);
                }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .child(div().truncate().text_color(rgb(TEXT)).child(title))
                            .child(
                                div()
                                    .mt_1()
                                    .text_sm()
                                    .text_color(rgb(MUTED))
                                    .truncate()
                                    .child(directory),
                            ),
                    )
                    .child(
                        div()
                            .ml_4()
                            .flex()
                            .flex_col()
                            .items_end()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(age)
                            .when_some(status, |element, status| {
                                element.child(
                                    div().mt_1().text_xs().text_color(rgb(ACCENT)).child(status),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let selected = self.timeline.session_id().map(ToOwned::to_owned);
        let statuses = Arc::clone(&self.statuses);
        let content = match &self.server_state {
            ServerState::Loading => centered_message("Connecting to OpenCode..."),
            ServerState::Failed(error) => div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .px_5()
                .child(div().text_color(rgb(TEXT)).child("OpenCode is unavailable"))
                .child(div().text_sm().text_color(rgb(MUTED)).child(error.clone()))
                .into_any_element(),
            ServerState::Ready { sessions, .. } => {
                let sessions = Arc::clone(sessions);
                uniform_list(
                    "sessions",
                    sessions.len(),
                    cx.processor(
                        move |_workspace, range: std::ops::Range<usize>, _window, cx| {
                            range
                                .map(|index| {
                                    let session = &sessions[index];
                                    Self::render_session(
                                        session,
                                        selected.as_deref() == Some(session.id.as_str()),
                                        statuses.get(&session.id),
                                        cx,
                                    )
                                })
                                .collect()
                        },
                    ),
                )
                .h_full()
                .into_any_element()
            }
        };

        div()
            .w(px(380.0))
            .h_full()
            .flex_none()
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(content)
            .into_any_element()
    }

    fn render_timeline(&self) -> gpui::AnyElement {
        match &self.timeline {
            TimelineState::Empty => centered_message("Select a session to inspect its timeline"),
            TimelineState::Loading { .. } => centered_message("Loading timeline..."),
            TimelineState::Failed { error, .. } => div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .child(div().text_color(rgb(TEXT)).child("Timeline unavailable"))
                .child(div().text_sm().text_color(rgb(MUTED)).child(error.clone()))
                .into_any_element(),
            TimelineState::Ready { messages, .. } if messages.is_empty() => {
                centered_message("This session has no messages")
            }
            TimelineState::Ready { messages, .. } => div()
                .id("timeline")
                .size_full()
                .overflow_y_scroll()
                .px_6()
                .py_5()
                .children(messages.iter().map(render_message))
                .into_any_element(),
        }
    }

    fn render_header(&self) -> gpui::AnyElement {
        let (status, count, status_color): (SharedString, Option<usize>, u32) =
            match &self.server_state {
                ServerState::Loading => ("connecting".into(), None, WARNING),
                ServerState::Failed(_) => ("offline".into(), None, DANGER),
                ServerState::Ready { version, sessions } => {
                    let label = if self.live {
                        format!("opencode {version}")
                    } else {
                        format!("opencode {version} · reconnecting")
                    };
                    (
                        label.into(),
                        Some(sessions.len()),
                        if self.live { ACCENT } else { WARNING },
                    )
                }
            };
        let context = self.timeline.title().unwrap_or_else(|| "sessions".into());

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
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().text_lg().text_color(rgb(TEXT)).child("opencode"))
                    .child(
                        div()
                            .max_w(px(560.0))
                            .truncate()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(context),
                    ),
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
                    .child(div().size_2().rounded_full().bg(rgb(status_color)))
                    .child(status),
            )
            .into_any_element()
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let header = self.render_header();
        let timeline = self.render_timeline();
        let sidebar = self.render_sidebar(cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(BACKGROUND))
            .font_family("Inter")
            .child(header)
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .flex()
                    .child(sidebar)
                    .child(div().min_w_0().flex_1().child(timeline)),
            )
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

fn render_message(message: &MessageRecord) -> gpui::AnyElement {
    let role: SharedString = message.info.role().into();
    let detail: SharedString = message.info.detail().into();
    let message_id: SharedString = message.info.id().to_owned().into();
    let parts = message
        .parts
        .iter()
        .filter_map(render_part)
        .collect::<Vec<_>>();

    div()
        .id(message_id)
        .mb_4()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .child(
            div()
                .mb_3()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_sm().text_color(rgb(ACCENT)).child(role))
                .child(div().text_xs().text_color(rgb(MUTED)).child(detail)),
        )
        .children(parts)
        .into_any_element()
}

fn render_part(part: &Part) -> Option<gpui::AnyElement> {
    let summary: SharedString = part.summary()?.into();
    let is_text = part.kind == "text";
    Some(
        div()
            .mb_2()
            .text_sm()
            .line_height(px(21.0))
            .text_color(rgb(if is_text { TEXT } else { MUTED }))
            .when(!is_text, |element| {
                element.px_3().py_2().rounded_md().bg(rgb(BACKGROUND))
            })
            .child(summary)
            .into_any_element(),
    )
}

fn centered_message(message: &'static str) -> gpui::AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(MUTED))
        .child(message)
        .into_any_element()
}

fn display_title(session: &Session) -> String {
    if session.title.trim().is_empty() {
        "Untitled session".into()
    } else {
        session.title.clone()
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
        let bounds = Bounds::centered(None, size(px(1180.0), px(780.0)), cx);
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
            |window, cx| {
                window.set_window_title("OpenCode");
                cx.new(Workspace::new)
            },
        )
        .expect("failed to open the OpenCode window");
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests {
    use opencode_gpui::model::{Message, SessionTime};

    use super::*;

    fn session(id: &str, updated: u64) -> Session {
        Session {
            id: id.into(),
            project_id: "project".into(),
            directory: "/workspace".into(),
            parent_id: None,
            title: id.into(),
            version: "1.18.16".into(),
            time: SessionTime {
                created: 1,
                updated,
                compacting: None,
            },
        }
    }

    fn workspace(sessions: Vec<Session>, timeline: TimelineState) -> Workspace {
        Workspace {
            client: None,
            server: "test".into(),
            server_state: ServerState::Ready {
                version: "1.18.16".into(),
                sessions: Arc::new(sessions),
            },
            timeline,
            statuses: Arc::new(HashMap::new()),
            live: true,
            _load: Task::ready(()),
            _events: Task::ready(()),
            timeline_load: None,
        }
    }

    #[test]
    fn reduces_session_updates_without_losing_sort_order() {
        let mut workspace = workspace(vec![session("a", 1), session("b", 2)], TimelineState::Empty);

        workspace.apply_events(vec![
            Event::SessionUpdated(session("a", 3)),
            Event::SessionStatus {
                session_id: "a".into(),
                status: SessionStatus::Busy,
            },
        ]);

        let ServerState::Ready { sessions, .. } = &workspace.server_state else {
            panic!("server should remain ready");
        };
        assert_eq!(
            sessions
                .iter()
                .map(|session| session.id.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(workspace.statuses["a"], SessionStatus::Busy);
    }

    #[test]
    fn reduces_streamed_message_parts_in_place() {
        let message: Message = serde_json::from_str(
            r#"{"id":"msg_1","sessionID":"ses_1","role":"user","time":{"created":1},"agent":"build","model":{"providerID":"openai","modelID":"test"}}"#,
        )
        .unwrap();
        let first: Part = serde_json::from_str(
            r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"text","text":"hel"}"#,
        )
        .unwrap();
        let complete: Part = serde_json::from_str(
            r#"{"id":"part_1","sessionID":"ses_1","messageID":"msg_1","type":"text","text":"hello"}"#,
        )
        .unwrap();
        let mut workspace = workspace(
            Vec::new(),
            TimelineState::Ready {
                session_id: "ses_1".into(),
                title: "session".into(),
                messages: Vec::new(),
            },
        );

        workspace.apply_events(vec![
            Event::MessageUpdated(message),
            Event::MessagePartUpdated {
                part: first,
                delta: Some("hel".into()),
            },
            Event::MessagePartUpdated {
                part: complete,
                delta: Some("lo".into()),
            },
        ]);

        let TimelineState::Ready { messages, .. } = &workspace.timeline else {
            panic!("timeline should remain loaded");
        };
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].parts.len(), 1);
        assert_eq!(messages[0].parts[0].text(), Some("hello"));
    }
}
