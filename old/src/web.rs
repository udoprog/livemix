use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::mixer;

pub fn setup() -> (Handle, Server) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = Handle {
        inner: Arc::new(InnerHandle {
            shutdown: Mutex::new(Some(tx)),
        }),
    };
    let server = Server { shutdown: rx };
    (handle, server)
}

struct InnerHandle {
    shutdown: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

/// The handle to a server.
#[derive(Clone)]
pub struct Handle {
    inner: Arc<InnerHandle>,
}

impl Handle {
    /// Shutdown the server.
    pub fn shutdown(&self) {
        if let Ok(mut guard) = self.inner.shutdown.lock() {
            guard.take();
        }
    }
}

/// The server instance.
pub struct Server {
    shutdown: tokio::sync::oneshot::Receiver<()>,
}

impl Server {
    /// Run the server.
    pub async fn start(self, mixer: mixer::Handle) -> Result<()> {
        // build our application with a route
        let mut app = Router::new();
        app = app.route("/playback", post(create_playback));
        app = app.layer(Extension(mixer));

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

        let shutdown = async move {
            _ = self.shutdown.await;
        };

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown)
            .await?;

        Ok(())
    }
}

#[axum::debug_handler]
async fn create_playback(
    Extension(mixer): Extension<mixer::Handle>,
    Json(payload): Json<CreatePlayback>,
) -> (StatusCode, Json<Playback>) {
    let user = Playback { id: 1337 };

    tracing::info!("Got request");

    mixer.send(mixer::Task::AddPlaybackStream);

    (StatusCode::CREATED, Json(user))
}

#[derive(Deserialize)]
struct CreatePlayback {}

#[derive(Serialize)]
struct Playback {
    id: u64,
}
