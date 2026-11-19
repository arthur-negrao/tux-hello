use tokio::io;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, error, info, warn};

use super::Task;
use super::listener::Listener;
use super::state::State;
use super::worker::spawn_worker;
use super::{RejectReason, Response};
use crate::discover::{CameraCandidate, Discover};
use crate::setup;

pub struct Application {
    pub state: State,
    pub discover: Option<Discover>,
    pub camera: Option<CameraCandidate>,
    worker_is_busy: bool,
    task_receiver: mpsc::Receiver<Task>,
    listener_handle: Option<JoinHandle<()>>,
}

impl Application {
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
                            "Error to listen the clients: {}",
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

    pub async fn run(&mut self) -> io::Result<()> {
        loop {
            match self.state {
                State::Discover => {
                    let Some(mut discover) = self.discover.take() else {
                        error!(
                            "Error with discover: Discover can not be None."
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
                                "Error to discover a camera: {}",
                                e.to_string()
                            );
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                format!(
                                    "Error to find new cameras: {}",
                                    e.to_string()
                                ),
                            ));
                        }
                    };
                }
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
                    debug!("Application starting the StandBy mode");

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
                State::Server => loop {
                    if let Some(task) = self.listen_tasks().await {
                        if self.worker_is_busy {
                            if let Err(_) = task.reply_to.send(
                                Response::reject(RejectReason::Busy, None),
                            ) {
                                warn!("Error to sent a busy response to task.");
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

    async fn listen_tasks(&mut self) -> Option<Task> {
        self.task_receiver.recv().await
    }

    #[inline]
    fn add_to_blacklist(&mut self) {
        if let Some(discover) = &mut self.discover {
            discover.add_to_blacklist(&self.camera);
        } else {
            error!(
                "Error: Can not add to blacklist, because the Discover is None"
            );
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

/// Run a `job` while responses a [`Task`] received by the `task_receiver` with
/// a `response`.
async fn run_while_responding<F, T>(
    job: F,
    task_receiver: &mut mpsc::Receiver<Task>,
    response: Response,
) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::pin!(job);

    let mut channel_is_open = !task_receiver.is_closed();

    loop {
        tokio::select! {
            res = &mut job => return res,

            // avoid checks when the channel is closed. This optimization avoid
            // busy waits.
            task_opt = task_receiver.recv(), if channel_is_open => {
                match task_opt {
                    Some(task) => {
                        let response_clone = response.clone();
                        let _ = task.reply_to.send(response_clone);
                    },
                    None => {
                        channel_is_open = false;
                    }
                }

            }
        }
    }
}
