use crate::{error::SourisError, state::SourisState};
use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct NewDB {
    name: String,
    overwrite_existing: bool,
}

pub async fn new_db(
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
