use tokio::sync::mpsc::channel;

use tux_hello::app::Application;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let (task_sender, task_receiver) = channel(32);

    let mut app = Application::new(task_receiver);

    app.start_server(task_sender).await?;

    app.run().await;

    Ok(())
}
