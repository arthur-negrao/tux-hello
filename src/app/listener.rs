use std::path::PathBuf;

use tokio::net::{UnixListener, UnixStream, unix::SocketAddr};
use tokio::sync::mpsc::Sender;

use super::task::{Task, dispatch_task};

/// A watcher to response the [`tokio::net::UnixListener`] requests.
pub struct Listener {
    listener: UnixListener,
    task_sender: Sender<Task>,
}

impl Listener {
    /// Create a new [`Listener`] and start the Unix Socket.
    pub async fn new(task_sender: Sender<Task>) -> tokio::io::Result<Self> {
        let path = PathBuf::from("/run/tux-hello/tux-hello.sock");
        _ = tokio::fs::remove_dir(&path).await;

        let listener = UnixListener::bind(path)?;

        Ok(Self {
            listener,
            task_sender,
        })
    }

    /// Listen the [`tokio::net::UnixListener`].
    #[inline]
    pub async fn listen(&self) -> tokio::io::Result<(UnixStream, SocketAddr)> {
        self.listener.accept().await
    }

    /// Dispatch a connection spawning a new task.
    pub fn dispatch_connection(&self, stream: UnixStream) {
        let sender = self.task_sender.clone();
        dispatch_task(stream, sender);
    }
}
