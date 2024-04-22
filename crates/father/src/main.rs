#[macro_use]
extern crate tracing;

use tracing::Level;
use crate::state::State;

mod state;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(Level::TRACE).init();
    
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
    
    let state = State::new().await.expect("unable to create state");
    info!("Found state {state:?}");



    state.save().await.expect("unable to save state.");
}
