use async_channel::{Receiver, Sender};
use tokio::task::JoinHandle;

use super::request::{Request, Response};

pub(super) enum WorkerCMD {}

pub fn spawn_worker(
    receiver: Receiver<Request>,
    sender: Sender<Response>,
) -> JoinHandle<()> {
    // TODO: Create the worker life cycle
    tokio::spawn(async move {
        while let Ok(request) = receiver.recv().await {
            match request {
                Request::Unlock { user } => {
                    _ = sender.send(Response::Accept);
                }
                Request::Enroll { user } => {
                    _ = sender.send(Response::Accept);
                }
            };
        }
    })
}
