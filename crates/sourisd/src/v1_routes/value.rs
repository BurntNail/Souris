use crate::{error::SourisError, state::SourisState};
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sourisdb::{utilities::cursor::Cursor, values::Value};

#[derive(Deserialize)]
pub struct KeyAndDb {
    pub db: String,
    pub key: String,
}

#[axum::debug_handler]
pub async fn add_kv(
    Query(KeyAndDb { db, key }): Query<KeyAndDb>,
    State(state): State<SourisState>,
    body: Bytes,
) -> Result<StatusCode, SourisError> {
    let v = Value::deser(&mut Cursor::new(&body))?;

    state.add_key_value_pair(db, key, v).await;
    Ok(StatusCode::CREATED)
}

#[axum::debug_handler]
pub async fn get_value(
    Query(KeyAndDb { key, db }): Query<KeyAndDb>,
    State(state): State<SourisState>,
) -> Result<Vec<u8>, SourisError> {
    let v = state.get_value(db, &key).await?;
    let bytes = v.ser()?;
    Ok(bytes)
}

#[axum::debug_handler]
pub async fn rm_key(
    Query(KeyAndDb { key, db }): Query<KeyAndDb>,
    State(state): State<SourisState>,
) -> Result<StatusCode, SourisError> {
    state.rm_key(key, db).await?;

    Ok(StatusCode::OK)
}
