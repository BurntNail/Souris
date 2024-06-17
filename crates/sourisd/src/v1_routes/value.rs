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
    Query(KeyAndDb { db_name: db, key }): Query<KeyAndDb>,
    State(state): State<SourisState>,
    value: Value,
) -> StatusCode {
    state.add_key_value_pair(db, key, value).await
}

#[axum::debug_handler]
pub async fn get_value(
    Query(KeyAndDb { key, db_name: db }): Query<KeyAndDb>,
    State(state): State<SourisState>,
) -> Result<Value, SourisError> {
    state.get_value(db, &key).await
}

#[axum::debug_handler]
pub async fn rm_key(
    Query(KeyAndDb { key, db_name: db }): Query<KeyAndDb>,
    State(state): State<SourisState>,
) -> Result<StatusCode, SourisError> {
    state.remove_key(key, db).await?;

    Ok(StatusCode::OK)
}
