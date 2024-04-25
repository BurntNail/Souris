#[macro_use]
extern crate tracing;

use crate::{
    apidoc::ApiDoc,
    state::SourisState,
    v1_routes::db::{add_db, clear_db, get_db, remove_db},
};
use axum::{
    routing::{get, post},
    Router,
};
use std::time::Duration;
use axum::routing::put;
use tokio::{
    net::TcpListener,
    signal,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::v1_routes::value::{add_kv, get_value};

mod apidoc;
mod error;
mod state;
mod v1_routes;

fn setup() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .with_span_events(FmtSpan::NEW)
        .init();
    color_eyre::install().expect("unable to install color-eyre");

    if cfg!(debug_assertions) {
        const TO: &str = "full";
        for key in &["RUST_SPANTRACE", "RUST_LIB_BACKTRACE", "RUST_BACKTRACE"] {
            match std::env::var(key) {
                Err(_) => {
                    trace!(%key, %TO, "Setting env var");
                    std::env::set_var(key, "full");
                }
                Ok(found) => {
                    trace!(%key, %found, "Found existing env var");
                }
            }
        }
    }
}

//from https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal(stop_signal: UnboundedSender<()>, saver: JoinHandle<()>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    };

    info!("Gracefully Exiting");
    stop_signal.send(()).expect("unable to send stop signal");
    saver.await.expect("unable to stop saver thread");
}

#[tokio::main]
async fn main() {
    setup();

    let state = SourisState::new().await.expect("unable to create state");
    info!("Found state {state:?}");

    let (stop_tx, mut stop_rx) = unbounded_channel();
    let saver_state = state.clone();
    let saver = tokio::task::spawn(async move {
        let state = saver_state;
        loop {
            tokio::select! {
                _ = stop_rx.recv() => {
                    info!("Stop signal received for saver");
                    break;
                },
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    if let Err(e) = state.save().await {
                        error!(?e, "Error saving state");
                    }
                }
            }
        }

        if let Err(e) = state.save().await {
            error!(?e, "Error saving state");
        }
        info!("Exiting saver");
    });

    let v1_router = Router::new()
        .route("/get_db", get(get_db))
        .route("/add_db", post(add_db))
        .route("/rm_db", post(remove_db))
        .route("/clear_db", post(clear_db))
        .route("/add_kv", put(add_kv))
        .route("/get_value", get(get_value));

    let router = Router::new()
        .nest("/v1", v1_router)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:2256").await.unwrap();

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal(stop_tx, saver))
        .await
        .unwrap();
}
