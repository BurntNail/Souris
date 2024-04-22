use axum::Json;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(openapi, crate::v1_routes::new_db::add_db), components(schemas(crate::v1_routes::new_db::NewDB)))]
pub struct ApiDoc;

#[utoipa::path(
    get,
    path = "/openapi.json",
    responses(
        (status = 200, description = "API Documentation", body = ())
    )
)]
pub async fn openapi () -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}