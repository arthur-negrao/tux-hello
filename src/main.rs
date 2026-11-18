use tux_hello::{
    app::{Application, State},
    setup,
};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let mut app = Application::new();

    loop {
        let current_state = app.read_state().await;

        match current_state {
            State::Discover => {
                if let Some(camera) = app.discover.discover() {
                    app.camera = Some(camera);
                    app.set_state(State::Setup).await;

                // no more devices
                } else {
                    app.set_state(State::StandBy).await;
                };
            }
            State::Setup => {
                // try configure the camera
                if setup::configure_camera(&mut app) {
                    app.set_state(State::Server).await;
                } else {
                    // avoid this camera in the next Discover and try again
                    // with new camera
                    app.add_to_blacklist();
                    app.set_state(State::Discover).await;
                }
            }
            State::StandBy => {
                break;
            }
            State::Server => {}
            State::Finish => {}
        }
    }

    Ok(())
}
