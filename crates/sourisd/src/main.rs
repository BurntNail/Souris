#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

#[macro_use]
extern crate tracing;

use crate::v1_routes::{
    db::{add_db, add_db_with_content, clear_db, get_all_dbs, get_db, remove_db},
    state::SourisState,
    value::{add_kv, get_value, rm_key},
};
use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post, put},
    Router,
};
use std::time::Duration;
use tokio::{
    net::TcpListener,
    signal,
    sync::{broadcast, broadcast::Sender},
    task::JoinHandle,
};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{fmt::format::FmtSpan, prelude::*, EnvFilter};

mod error;
mod v1_routes;

fn setup() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
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
async fn shutdown_signal(stop_signal: Sender<()>, saver: JoinHandle<()>) {
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
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("Gracefully Exiting");
    stop_signal.send(()).expect("unable to send stop signal");

    if let Err(e) = saver.await {
        error!(?e, "Unable to join saver thread");
    }
}

async fn healthcheck() -> StatusCode {
    StatusCode::OK
}

#[tokio::main]
async fn main() {
    setup();

    let state = SourisState::new().await.expect("unable to create state");
    info!("Found state {state:?}");

    let (stop_tx, stop_rx) = broadcast::channel(1);
    let saver_state = state.clone();

    let mut saver_stop_rx = stop_rx.resubscribe();
    let saver = tokio::task::spawn(async move {
        let state = saver_state;
        loop {
            tokio::select! {
                _ = saver_stop_rx.recv() => {
                    info!("Stop signal received for saver");
                    break;
                },
                () = tokio::time::sleep(Duration::from_secs(10)) => {
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
        .route("/get_all_dbs", get(get_all_dbs))
        .route("/add_db", post(add_db))
        .route("/add_db_with_content", put(add_db_with_content))
        .route("/rm_db", post(remove_db))
        .route("/clear_db", post(clear_db))
        .route("/add_kv", put(add_kv))
        .route("/rm_key", post(rm_key))
        .route("/get_value", get(get_value));

    let router = Router::new()
        .route("/healthcheck", get(healthcheck))
        .nest("/v1", v1_router)
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::disable())
        .with_state(state.clone());

    let http_listener = TcpListener::bind("127.0.0.1:2256").await.unwrap();

    axum::serve(http_listener, router)
        .with_graceful_shutdown(shutdown_signal(stop_tx, saver))
        .await
        .unwrap();
}
