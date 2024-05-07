use crate::{error::SourisError, state::SourisState, v1_routes::db::DbByName};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::Value as SJValue;
use sourisdb::values::Value;

#[derive(Deserialize)]
pub struct KeyAndValue {
    k: String,
    v: SJValue,
}

#[derive(Deserialize)]
pub struct KeyAndDb {
    pub db: String,
    pub key: String,
}

#[axum::debug_handler]
pub async fn add_kv(
    Query(DbByName { name }): Query<DbByName>,
    State(state): State<SourisState>,
    Json(KeyAndValue { k, v }): Json<KeyAndValue>,
) -> StatusCode {
    match state.add_key_value_pair(name, k, Value::from(v)).await {
        true => StatusCode::CREATED,
        false => StatusCode::OK,
    }
}

#[axum::debug_handler]
pub async fn get_value(
    Query(KeyAndDb { key, db }): Query<KeyAndDb>,
    State(state): State<SourisState>,
) -> Result<Json<Value>, SourisError> {
    state.get_value(db, &key).await.map(Json)
}
