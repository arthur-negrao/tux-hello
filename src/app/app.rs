use tokio::io;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, error, info, warn};

use super::state::State;
// use super::worker::spawn_worker;
use crate::discover::{CameraCandidate, Discover};
use crate::ipc::request::{RejectReason, Response, run_while_responding};
use crate::ipc::{Listener, Task};
use crate::setup;

/// The main loop to reply socket requests.
pub struct Application {
    pub state: State,
    pub discover: Option<Discover>,
    pub camera: Option<CameraCandidate>,
    worker_is_busy: bool,
    task_receiver: mpsc::Receiver<Task>,
    listener_handle: Option<JoinHandle<()>>,
}

impl Application {
    /// Creates a new [`Application`] instance.
    pub fn new(task_receiver: mpsc::Receiver<Task>) -> Self {
        Self {
            state: State::Discover,
            discover: Some(Discover::new()),
            camera: None,
            worker_is_busy: false,
            task_receiver,
            listener_handle: None,
        }
    }

    /// Starts the server to reply socket requests.
    pub async fn start_server(
        &mut self,
        task_sender: mpsc::Sender<Task>,
    ) -> tokio::io::Result<()> {
        let mut listener = Listener::new(task_sender).await?;

        let handle = tokio::spawn(async move {
            loop {
                match listener.listen().await {
                    Ok((stream, _)) => listener.dispatch_connection(stream),
                    Err(e) => {
                        error!(
                            "Failed to accept client connection: {}",
                            e.to_string()
                        );

                        // stops the listener
                        break;
                    }
                }
            }
        });

        self.listener_handle = Some(handle);

        Ok(())
    }

    /// Starts the application main loop.
    pub async fn run(&mut self) -> io::Result<()> {
        loop {
            match self.state {
                // TODO: Create a new function to simplify the discover code.
                State::Discover => {
                    let Some(mut discover) = self.discover.take() else {
                        error!(
                            "Discovery error: Discover instance cannot be None."
                        );
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Discover can not be None.",
                        ));
                    };

                    let job = tokio::task::spawn_blocking(move || {
                        let cam = discover.discover();
                        (discover, cam)
                    });

                    let job_result = run_while_responding(
                        async { job.await },
                        &mut self.task_receiver,
                        Response::reject(RejectReason::NoCamFound, None),
                    )
                    .await;

                    match job_result {
                        Ok((discover, camera_opt)) => {
                            self.discover = Some(discover);

                            if let Some(camera) = camera_opt {
                                self.camera = Some(camera);
                                self.state = State::Setup;

                            // no more devices
                            } else {
                                self.state = State::StandBy;
                            };
                        }
                        Err(e) => {
                            error!(
                                "Failed to discover camera: {}",
                                e.to_string()
                            );
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                format!(
                                    "Failed to find new cameras: {}",
                                    e.to_string()
                                ),
                            ));
                        }
                    };
                }
                // TODO: Create a function to avoid blocking configure_camera function.
                State::Setup => {
                    let result = run_while_responding(
                        // try configure the camera
                        async { setup::configure_camera(&mut self.camera) },
                        &mut self.task_receiver,
                        Response::reject(RejectReason::NoCamFound, None),
                    )
                    .await;

                    if result {
                        self.state = State::Server;
                    } else {
                        // avoid this camera in the next Discover and try again
                        // with new camera
                        self.add_to_blacklist();
                        self.state = State::Discover;
                    }
                }
                State::StandBy => {
                    debug!("Application entering StandBy mode");

                    run_while_responding(
                        async {
                            time::sleep(time::Duration::from_secs(10)).await
                        },
                        &mut self.task_receiver,
                        Response::reject(RejectReason::NoCamFound, None),
                    )
                    .await;

                    self.state = State::Discover;
                }
                // TODO: Create a main logic to the Server State.
                State::Server => loop {
                    if let Some(task) = self.listen_tasks().await {
                        if self.worker_is_busy {
                            if let Err(_) = task.reply_to.send(
                                Response::reject(RejectReason::Busy, None),
                            ) {
                                warn!("Failed to send busy response to task");
                            }
                        }
                    } else {
                        self.state = State::Finish;
                        break;
                    }
                },
                State::Finish => {
                    info!("Finishing the program.");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Listen [`Task`]s until the channel closes.
    async fn listen_tasks(&mut self) -> Option<Task> {
        self.task_receiver.recv().await
    }

    /// Add a camera into the blacklist to avoid it.
    #[inline]
    fn add_to_blacklist(&mut self) {
        if let Some(discover) = &mut self.discover {
            discover.add_to_blacklist(&self.camera);
        } else {
            error!("Cannot add to blacklist because Discover is None");
        }
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        // cleanup the listener task
        if let Some(handle) = self.listener_handle.take() {
            tracing::trace!("Aborting the Listener task.");
            handle.abort();
        }
    }
}
