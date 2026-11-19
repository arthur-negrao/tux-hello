use tokio::{
    net::UnixStream,
    sync::{mpsc, oneshot},
};
use tracing::{debug, error, warn};

use super::decoder::IPCConnection;
use super::request::{RejectReason, Request, Response};

#[derive(Debug)]
pub struct Task {
    pub request: Request,
    pub reply_to: oneshot::Sender<Response>,
}

/// Dispatch a new task to talk to client.
///
/// Uses the `stream` to read requests and send the response. Futhermore, the
/// `sender` will be used to send the [`Task`] to the [`super::Application`]
pub(super) fn dispatch_task(
    task_id: u64,
    stream: UnixStream,
    sender: mpsc::Sender<Task>,
) {
    tokio::spawn(async move {
        handle_client(task_id, stream, sender).await;
        debug!("Finishing the task: {}", task_id);
    });
}

#[tracing::instrument(skip(stream, sender))]
async fn handle_client(
    task_id: u64,
    stream: UnixStream,
    sender: mpsc::Sender<Task>,
) {
    debug!("Starting the task");

    // use the stream to create a new connection
    let mut connection = IPCConnection::new(Some(stream));

    // a oneshot channel to talk to application
    let (response_sender, response_receiver) = oneshot::channel::<Response>();

    match connection.read_request().await {
        Ok(r) => {
            let task = Task {
                request: r,
                reply_to: response_sender,
            };

            // the application task was killed by a panic!()
            if let Err(e) = sender.send(task).await {
                error!(
                    "Error to send a task to application: {}",
                    e.to_string()
                );
                return;
            }

            match response_receiver.await {
                Ok(response) => {
                    _ = connection
                        .write_response(response)
                        .await
                        .inspect_err(|e| {
                            warn!(
                                "Error to send a response to client: {}",
                                e.to_string()
                            );
                        });
                }
                Err(response_error) => {
                    _ = connection
                            .write_response(Response::reject(
                                RejectReason::InternalError,
                                Some(format!(
                                    "Error to receive a response from application: {}",
                                    response_error.to_string()
                                )),
                            ))
                            .await
                            .inspect_err(|e| {
                                warn!(
                                    "Error to send a reponse to client: {}", 
                                    e.to_string()
                                );
                            });
                }
            }
        }
        Err(read_error) => {
            warn!("Fail to read the request: {}", read_error.to_string());

            _ = connection
                .write_response(Response::reject(
                    RejectReason::InternalError,
                    Some(format!(
                        "Fail to read the request: {}",
                        read_error.to_string()
                    )),
                ))
                .await
                .inspect_err(|e| {
                    warn!(
                        "Error to send a response to client: {}",
                        e.to_string()
                    );
                });
        }
    }
}
