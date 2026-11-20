use std::path::PathBuf;

use tokio::net::{UnixListener, UnixStream, unix::SocketAddr};
use tokio::sync::mpsc::Sender;
use tracing::debug;

use super::task::{Task, dispatch_task};

/// A watcher that accepts and dispatches [`tokio::net::UnixListener`] connections.
pub struct Listener {
    listener: UnixListener,
    task_sender: Sender<Task>,
    task_counter: u64,
}

impl Listener {
    /// Create a new [`Listener`] and start the Unix Socket.
    pub async fn new(task_sender: Sender<Task>) -> tokio::io::Result<Self> {
        let path = std::env::var("TUX_HELLO_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/run/tux-hello/tux-hello.sock"));

        debug!("The socket path is: {:?}", path);

        debug!("Removing the old socket: {:?}", path);
        _ = tokio::fs::remove_file(&path).await;

        debug!("Binding socket to: {:?}", path);
        let listener = UnixListener::bind(path)?;

        Ok(Self {
            listener,
            task_sender,
            task_counter: 0,
        })
    }

    /// Listen the [`tokio::net::UnixListener`].
    #[inline]
    pub async fn listen(&self) -> tokio::io::Result<(UnixStream, SocketAddr)> {
        self.listener.accept().await
    }

    /// Dispatch a connection spawning a new task.
    pub fn dispatch_connection(&mut self, stream: UnixStream) {
        let current_id = self.task_counter;

        let sender = self.task_sender.clone();

        dispatch_task(current_id, stream, sender);

        self.task_counter += self.task_counter.wrapping_add(1);
    }
}
