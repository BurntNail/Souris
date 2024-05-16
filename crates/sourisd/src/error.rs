use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sourisdb::{store::StoreSerError, values::ValueSerError};
use std::{
    error::Error,
    fmt::{Display, Formatter},
};
use tokio::io::Error as IOError;

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum SourisError {
    IO(IOError),
    DatabaseNotFound,
    KeyNotFound,
    StoreError(StoreSerError),
    ValueError(ValueSerError),
    InvalidDatabaseName,
}

impl From<IOError> for SourisError {
    fn from(value: IOError) -> Self {
        Self::IO(value)
    }
}
impl From<StoreSerError> for SourisError {
    fn from(value: StoreSerError) -> Self {
        Self::StoreError(value)
    }
}
impl From<ValueSerError> for SourisError {
    fn from(value: ValueSerError) -> Self {
        Self::ValueError(value)
    }
}

impl Error for SourisError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IO(e) => Some(e),
            Self::StoreError(e) => Some(e),
            Self::ValueError(e) => Some(e),
            _ => None,
        }
    }
}

impl Display for SourisError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StoreError(e) => write!(f, "Error with Souris Store: {e}"),
            Self::IO(e) => write!(f, "Error with IO: {e}"),
            Self::DatabaseNotFound => write!(f, "Could not find database with name"),
            Self::KeyNotFound => write!(f, "Could not find value with name in database provided"),
            Self::ValueError(e) => write!(f, "Error with value: {e}"),
            Self::InvalidDatabaseName => write!(f, "Invalid database name - database names must be ASCII and not equal to `meta`"),
        }
    }
}

impl IntoResponse for SourisError {
    fn into_response(self) -> Response {
        error!(?self, "Returning error");

        let code = match self {
            Self::DatabaseNotFound | Self::KeyNotFound => StatusCode::NOT_FOUND,
            Self::InvalidDatabaseName => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (code, format!("{self}")).into_response()
    }
}
