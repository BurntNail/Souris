use crate::{error::SourisError, state::SourisState};
use axum::{
    extract::{Query, State},
    body::Bytes,
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
    Query(NewDB {
        name,
        overwrite_existing,
    }): Query<NewDB>,
) -> Result<StatusCode, SourisError> {
    state.new_db(name.clone(), overwrite_existing).await
}

pub async fn add_db_with_content (
    State(state): State<SourisState>,
    Query(NewDB {
             name,
             overwrite_existing,
         }): Query<NewDB>,
    body: Bytes
) -> Result<StatusCode, SourisError> {
    let store = Store::deser(body.as_ref())?;
    Ok(state.new_db_with_contents(name, overwrite_existing, store).await)
}

pub async fn clear_db(
    State(state): State<SourisState>,
    Query(DbByName { name }): Query<DbByName>,
) -> Result<StatusCode, SourisError> {
    state.clear_db(name).await?;
    Ok(StatusCode::OK)
}

pub async fn remove_db(
    State(state): State<SourisState>,
    Query(DbByName { name }): Query<DbByName>,
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
