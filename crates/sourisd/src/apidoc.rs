use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::v1_routes::db::add_db,
        crate::v1_routes::db::clear_db,
        crate::v1_routes::db::remove_db,
        crate::v1_routes::db::get_db
    ),
    components(schemas(crate::v1_routes::db::NewDB, crate::v1_routes::db::DbByName))
)]
pub struct ApiDoc;
