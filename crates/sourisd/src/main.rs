#[macro_use]
extern crate tracing;

use crate::{v1_routes::new_db::add_db, state::SourisState};
use axum::{routing::post, Router};
use axum::routing::get;
use tokio::{net::TcpListener, signal};
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::apidoc::{ApiDoc, openapi};

mod error;
mod v1_routes;
mod state;
mod apidoc;

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
async fn shutdown_signal(state: SourisState) {
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

    if let Err(e) = state.save().await {
        error!(?e, "Error saving state.");
    }
}

#[tokio::main]
async fn main() {
    setup();

    let state = SourisState::new().await.expect("unable to create state");
    info!("Found state {state:?}");

    let v1_router = Router::new().route("/add_db", post(add_db));

    let router = Router::new()
        .nest("/v1_routes", v1_router)
        .route("/openapi.json", get(openapi))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:2256").await.unwrap();

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal(state))
        .await
        .unwrap();
}
