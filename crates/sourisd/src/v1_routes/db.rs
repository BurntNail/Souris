use axum::{
    body::Bytes,
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use sourisdb::store::Store;

use crate::{error::SourisError, v1_routes::state::SourisState};

#[derive(Deserialize)]
pub struct NewDB {
    pub db_name: String,
    pub overwrite_existing: bool,
}

#[derive(Deserialize)]
pub struct DbByName {
    pub db_name: String,
}

pub async fn add_db(
    State(state): State<SourisState>,
    Query(NewDB {
        db_name: name,
        overwrite_existing,
    }): Query<NewDB>,
) -> Result<StatusCode, SourisError> {
    state.new_db(name, overwrite_existing).await
}

pub async fn add_db_with_content(
    State(state): State<SourisState>,
    Query(NewDB {
        db_name: name,
        overwrite_existing,
    }): Query<NewDB>,
    body: Bytes,
) -> Result<StatusCode, SourisError> {
    let store = Store::deser(body.as_ref())?;
    Ok(state
        .new_db_with_contents(name, overwrite_existing, store)
        .await)
}

pub async fn clear_db(
    State(state): State<SourisState>,
    Query(DbByName { db_name: name }): Query<DbByName>,
) -> Result<StatusCode, SourisError> {
    state.clear_db(name).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_db(
    State(state): State<SourisState>,
    Query(DbByName { db_name: name }): Query<DbByName>,
) -> Result<StatusCode, SourisError> {
    state.remove_db(name).await?;
    Ok(StatusCode::OK)
}

#[axum::debug_handler]
pub async fn get_db(
    State(state): State<SourisState>,
    Query(DbByName { db_name: name }): Query<DbByName>,
) -> Result<Bytes, SourisError> {
    state.get_db(name).await
}

pub async fn get_all_dbs(State(state): State<SourisState>) -> Json<Vec<String>> {
    Json(state.get_all_db_names().await)
}
