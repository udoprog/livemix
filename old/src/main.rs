use std::pin::pin;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    livemix::pw::init();

    let (web_handle, web) = livemix::web::setup();
    let (mixer_handle, mixer) = livemix::mixer::setup();

    let mut mixer = tokio::task::spawn_blocking(move || mixer.run());

    let mut is_mixer_running = true;
    let mut is_web_running = true;

    let mut web = pin!(web.start(mixer_handle.clone()));

    while is_mixer_running || is_web_running {
        tokio::select! {
            _ = &mut mixer, if is_mixer_running => {
                tracing::info!("Mixer task ended.");
                is_mixer_running = false;
            }
            _ = web.as_mut(), if is_web_running => {
                tracing::info!("Web task ended.");
                is_web_running = false;
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl+C, shutting down...");
                web_handle.shutdown();
                mixer_handle.shutdown();
            }
        }
    }
}
