use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use tokio::sync::oneshot;

use crate::event::{Event, Payload};

use super::{ClientInner, Error, EventSubscription};

impl ClientInner {
    pub(super) async fn subscribe_events(&self) -> Result<EventSubscription, Error> {
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
