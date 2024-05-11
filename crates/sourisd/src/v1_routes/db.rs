use crate::{error::SourisError, state::SourisState};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct NewDB {
    pub name: String,
    pub overwrite_existing: bool,
}

#[derive(Deserialize)]
pub struct DbByName {
    pub name: String,
}

pub async fn add_db(
    State(state): State<SourisState>,
    Json(NewDB {
        name,
        overwrite_existing,
    }): Json<NewDB>,
) -> Result<StatusCode, SourisError> {
    state.new_db(name.clone(), overwrite_existing).await
}

pub async fn clear_db(
    State(state): State<SourisState>,
    Json(DbByName { name }): Json<DbByName>,
) -> Result<StatusCode, SourisError> {
    state.clear_db(name).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_db(
    State(state): State<SourisState>,
    Json(DbByName { name }): Json<DbByName>,
) -> Result<StatusCode, SourisError> {
    state.remove_db(name).await?;
    Ok(StatusCode::OK)
}

#[axum::debug_handler]
pub async fn get_db(
    State(state): State<SourisState>,
    Query(DbByName { name }): Query<DbByName>,
) -> Result<Vec<u8>, SourisError> {
    let db = state.get_db(name).await?;
    let bytes = db.ser()?;
    Ok(bytes)
}
