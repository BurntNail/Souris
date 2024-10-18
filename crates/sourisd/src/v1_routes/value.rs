use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use sourisdb::values::Value;

use crate::{error::SourisError, v1_routes::state::SourisState};

#[derive(Deserialize)]
pub struct KeyAndDb {
    pub db_name: String,
    pub key: String,
}

#[axum::debug_handler]
pub async fn add_kv(
    Query(kanddb): Query<KeyAndDb>,
    State(state): State<SourisState>,
    value: Value,
) -> StatusCode {
    info!(?value, "Adding value");
    state.add_key_value_pair(kanddb, value).await
}

#[axum::debug_handler]
pub async fn get_value(
    Query(kanddb): Query<KeyAndDb>,
    State(state): State<SourisState>,
) -> Result<Value, SourisError> {
    state.get_value(kanddb).await
}

#[axum::debug_handler]
pub async fn rm_key(
    Query(kanddb): Query<KeyAndDb>,
    State(state): State<SourisState>,
) -> Result<StatusCode, SourisError> {
    state.remove_key(kanddb).await?;

    Ok(StatusCode::OK)
}
