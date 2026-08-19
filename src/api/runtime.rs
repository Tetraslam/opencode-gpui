use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};

use tokio::{runtime::Handle, sync::oneshot};

use super::Error;

#[derive(Clone)]
pub(super) struct Runtime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    handle: Handle,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
}

impl Runtime {
    pub(super) fn new() -> Result<Self, Error> {
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

    pub(super) async fn run<T: Send + 'static>(
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
