use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sourisdb::store::StoreError;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};
use tokio::io::Error as IOError;

#[derive(Debug)]
pub enum SourisError {
    StoreError(StoreError),
    IO(IOError),
    DatabaseNotFound,
    KeyNotFound,
}

impl From<StoreError> for SourisError {
    fn from(value: StoreError) -> Self {
        Self::StoreError(value)
    }
}
impl From<IOError> for SourisError {
    fn from(value: IOError) -> Self {
        Self::IO(value)
    }
}

impl Error for SourisError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::StoreError(e) => Some(e),
            Self::IO(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for SourisError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StoreError(e) => write!(f, "Error with Souris Store: {e:?}"),
            Self::IO(e) => write!(f, "Error with IO: {e:?}"),
            Self::DatabaseNotFound => write!(f, "Could not find database with name"),
            Self::KeyNotFound => write!(f, "Could not find value with name in database provided"),
        }
    }
}

impl IntoResponse for SourisError {
    fn into_response(self) -> Response {
        error!(?self, "Returning error");

        let code = match self {
            Self::IO(_) | Self::StoreError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::DatabaseNotFound | Self::KeyNotFound => StatusCode::NOT_FOUND,
        };

        (code, format!("{self:?}")).into_response()
    }
}
