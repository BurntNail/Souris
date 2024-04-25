use crate::{error::SourisError, state::SourisState};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sourisdb::store::Store;

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
    let found_existing = state.new_db(name.clone()).await?;

    Ok(if found_existing {
        if overwrite_existing {
            state.clear_db(name).await;
        }

        StatusCode::OK
    } else {
        StatusCode::CREATED
    })
}

pub async fn clear_db(
    State(state): State<SourisState>,
    Json(DbByName { name }): Json<DbByName>,
) -> StatusCode {
    if state.clear_db(name).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

pub async fn remove_db(
    State(state): State<SourisState>,
    Json(DbByName { name }): Json<DbByName>,
) -> Result<StatusCode, SourisError> {
    Ok(if state.remove_db(name).await? {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    })
}

#[axum::debug_handler]
pub async fn get_db(
    State(state): State<SourisState>,
    Query(DbByName { name }): Query<DbByName>,
) -> Result<Json<Store>, SourisError> {
    match state.get_db(name).await {
        Some(db) => Ok(Json(db)),
        None => Err(SourisError::DatabaseNotFound),
    }
}
