use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use crate::v1_routes::db::DbByName;
use crate::error::SourisError;
use crate::state::SourisState;
use serde_json::Value as SJValue;
use utoipa::{IntoParams, ToSchema};
use sourisdb::values::Value;

#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"key": "todo", "value": "todo"}))]
pub struct KeyAndValue {
    k: String,
    v: SJValue
}

#[derive(Deserialize, ToSchema, IntoParams)]
#[schema(example = json!({"db": "my_database", "key": "my_key"}))]
pub struct KeyAndDb {
    pub db: String,
    pub key: String
}

#[utoipa::path(
    post,
    path = "/v1/add_kv",
    request_body = KeyAndValue,
    responses(
        (status = OK, description = "Added"),
        (status = CREATED, description = "Created a new database & Added")
    )
)]
#[axum::debug_handler]
pub async fn add_kv (Query(DbByName {name}): Query<DbByName>, State(state): State<SourisState>, Json(KeyAndValue {k, v}): Json<KeyAndValue>) -> StatusCode {
    match state.add_key_value_pair(name, k, Value::from(v)).await {
        true => StatusCode::CREATED,
        false => StatusCode::OK
    }
}

#[utoipa::path(
    get,
    path = "/v1/get_key",
    request_body = SJValue,
    responses(
        (status = OK, description = "Found"),
        (status = NOT_FOUND, description = "Not found")
    )
)]
#[axum::debug_handler]
pub async fn get_value (Query(KeyAndDb {key, db}): Query<KeyAndDb>, State(state): State<SourisState>) -> Result<Json<Value>, SourisError> {
    state.get_value(db, &key).await.map(Json)
}