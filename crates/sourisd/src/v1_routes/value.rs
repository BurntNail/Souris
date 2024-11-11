use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use sourisdb::client::CreationResult;
use sourisdb::values::Value;

use crate::{error::SourisError, v1_routes::state::SourisState};

#[derive(Deserialize)]
pub struct KeyAndDb {
    pub db_name: String,
    pub key: String,
}

#[derive(Deserialize)]
pub struct NewKeyArgs {
    pub db_name: String,
    pub key: String,
    pub create_new_database: bool,
    pub overwrite_key: bool,
}

#[axum::debug_handler]
pub async fn add_kv(
    Query(nka): Query<NewKeyArgs>,
    State(state): State<SourisState>,
    value: Value,
) -> Json<CreationResult> {
    info!(?value, "Adding value");
    let cr = state.add_key_value_pair(nka, value).await;
    Json(cr)
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
