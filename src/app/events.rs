use std::{sync::Arc, time::Duration};

use gpui::{Context, Task, Timer};
use opencode_gpui::{
    api::Client,
    event::{Event, SessionStatus},
    model::{MessageRecord, Session, sort_sessions},
};

use super::{ServerState, TimelineState, Workspace};

impl Workspace {
    pub(super) fn spawn_event_loop(client: Option<Client>, cx: &Context<Self>) -> Task<()> {
        cx.spawn(async move |workspace, cx| {
            let Some(client) = client else {
                return;
            };
            let mut retry_delay = Duration::from_millis(250);
            loop {
                let subscription = client.subscribe_events().await;
                let Ok(mut subscription) = subscription else {
                    if workspace.update(cx, mark_disconnected).is_err() {
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

                if workspace.update(cx, mark_disconnected).is_err() {
                    return;
                }
                Timer::after(retry_delay).await;
                retry_delay = (retry_delay * 2).min(Duration::from_secs(8));
            }
        })
    }

    pub(super) fn apply_events(&mut self, events: Vec<Event>) {
        for event in events {
            match event {
                Event::ServerConnected | Event::Unknown(_) => {}
                Event::SessionCreated(session) | Event::SessionUpdated(session) => {
                    self.upsert_session(session);
                }
                Event::SessionDeleted(session) => self.delete_session(&session),
                Event::SessionStatus { session_id, status } => {
                    Arc::make_mut(&mut self.statuses).insert(session_id, status);
                }
                Event::SessionIdle { session_id } => {
                    Arc::make_mut(&mut self.statuses).insert(session_id, SessionStatus::Idle);
                }
                Event::MessageUpdated(info) => self.update_message(info),
                Event::MessageRemoved {
                    session_id,
                    message_id,
                } => self.remove_message(&session_id, &message_id),
                Event::MessagePartUpdated { part, .. } => self.update_part(part),
                Event::MessagePartRemoved {
                    session_id,
                    message_id,
                    part_id,
                } => self.remove_part(&session_id, &message_id, &part_id),
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

    fn delete_session(&mut self, session: &Session) {
        if let ServerState::Ready { sessions, .. } = &mut self.server_state {
            Arc::make_mut(sessions).retain(|current| current.id != session.id);
        }
        Arc::make_mut(&mut self.statuses).remove(&session.id);
        if self.timeline.session_id() == Some(session.id.as_str()) {
            self.timeline = TimelineState::Empty;
            self.timeline_load = None;
        }
    }

    fn update_message(&mut self, info: opencode_gpui::model::Message) {
        let TimelineState::Ready {
            session_id,
            messages,
            ..
        } = &mut self.timeline
        else {
            return;
        };
        if info.session_id() != session_id {
            return;
        }
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

    fn remove_message(&mut self, session_id: &str, message_id: &str) {
        if let TimelineState::Ready {
            session_id: selected,
            messages,
            ..
        } = &mut self.timeline
            && selected == session_id
        {
            messages.retain(|message| message.info.id() != message_id);
        }
    }

    fn update_part(&mut self, part: opencode_gpui::model::Part) {
        let TimelineState::Ready {
            session_id,
            messages,
            ..
        } = &mut self.timeline
        else {
            return;
        };
        if part.session_id != *session_id {
            return;
        }
        let Some(message) = messages
            .iter_mut()
            .find(|message| message.info.id() == part.message_id)
        else {
            return;
        };
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

    fn remove_part(&mut self, session_id: &str, message_id: &str, part_id: &str) {
        if let TimelineState::Ready {
            session_id: selected,
            messages,
            ..
        } = &mut self.timeline
            && selected == session_id
            && let Some(message) = messages
                .iter_mut()
                .find(|message| message.info.id() == message_id)
        {
            message.parts.retain(|part| part.id != part_id);
        }
    }
}

fn mark_disconnected(workspace: &mut Workspace, cx: &mut Context<Workspace>) {
    workspace.live = false;
    cx.notify();
}
