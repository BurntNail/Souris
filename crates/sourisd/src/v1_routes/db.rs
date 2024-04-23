use crate::{error::SourisError, state::SourisState};
use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({"name": "my database", "overwrite_existing": false}))]
pub struct NewDB {
    name: String,
    overwrite_existing: bool,
}

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({"name": "my database"}))]
pub struct DbByName {
    name: String,
}

#[utoipa::path(
    post,
    path = "/v1/add_db",
    request_body = NewDB,
    responses(
        (status = OK, description = "Found an existing database"),
        (status = CREATED, description = "Created a new database")
    )
)]
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

#[utoipa::path(
    post,
    path = "/v1/clear_db",
    request_body = DbByName,
    responses(
        (status = OK, description = "Found an existing database and cleared it"),
        (status = NOT_FOUND, description = "Unable to find the database")
    )
)]
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
