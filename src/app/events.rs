use std::time::Duration;

use gpui::{Context, Task, Timer};
use opencode_gpui::{api::Client, event::Event};

use super::Workspace;

impl Workspace {
    pub(super) fn spawn_event_loop(
        client: Client,
        directory: String,
        cx: &Context<Self>,
    ) -> Task<()> {
        cx.spawn(async move |workspace, cx| {
            let mut retry_delay = Duration::from_millis(250);
            loop {
                let subscription = client.subscribe_events().await;
                let Ok(mut subscription) = subscription else {
                    if workspace
                        .update(cx, |workspace, cx| {
                            workspace.mark_disconnected(&directory, cx);
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
                        workspace.connected_directories.insert(directory.clone());
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
                                workspace.merge_directory_sessions(&directory, bootstrap.sessions);
                            }
                            workspace.apply_events(batch, Some(&directory));
                            workspace.refresh_markdown(&directory, cx);
                            workspace.refresh_image_cache(&directory, cx);
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
                        workspace.mark_disconnected(&directory, cx);
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

    fn mark_disconnected(&mut self, directory: &str, cx: &mut Context<Self>) {
        self.connected_directories.remove(directory);
        cx.notify();
    }
}
