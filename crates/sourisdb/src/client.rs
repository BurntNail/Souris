use core::fmt::{Display, Formatter};

use http::{status::InvalidStatusCode, StatusCode};

#[cfg(feature = "async_client")]
pub use async_client::AsyncClient;
#[cfg(feature = "sync_client")]
pub use sync_client::SyncClient;

use crate::{
    store::StoreSerError,
    values::{ValueSerError, ValueTy},
};

#[cfg(feature = "async_client")]
mod async_client;
#[cfg(feature = "sync_client")]
mod sync_client;

#[derive(Debug)]
pub enum ClientError {
    #[cfg(feature = "sync_client")]
    Ureq(Box<ureq::Error>), //boxed because the error is *bigggg*
    #[cfg(feature = "async_client")]
    Reqwest(reqwest::Error),
    Store(StoreSerError),
    Value(ValueSerError),
    HttpErrorCode(StatusCode),
    IO(std::io::Error),
    InvalidStatusCode(InvalidStatusCode),
    ExpectedKey(&'static str),
    IncorrectType {
        ex: ValueTy,
        found: ValueTy,
    },
    ServerNotHealthy(StatusCode),
    SerdeJson(serde_json::Error),
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            #[cfg(feature = "sync_client")]
            Self::Ureq(u) => write!(f, "Error with ureq: {u}"),
            #[cfg(feature = "async_client")]
            Self::Reqwest(r) => write!(f, "Error with reqwest: {r}"),
            Self::Store(s) => write!(f, "Error with store: {s}"),
            Self::HttpErrorCode(sc) => write!(f, "Error with response: {sc:?}"),
            Self::IO(e) => write!(f, "IO Error: {e}"),
            Self::InvalidStatusCode(e) => write!(f, "Invalid status code provided: {e}"),
            Self::ExpectedKey(k) => write!(f, "Expected to find key: {k:?} in `Store` of body"),
            Self::IncorrectType { ex, found } => {
                write!(f, "Expected to find value of type {ex:?}, found {found:?}")
            }
            Self::ServerNotHealthy(sc) => write!(
                f,
                "Tried to get server health check, got status code: {sc:?}"
            ),
            Self::SerdeJson(e) => write!(f, "Tried to parse JSON and failed: {e}"),
            Self::Value(e) => write!(f, "Error with value: {e}"),
        }
    }
}

#[cfg(feature = "sync_client")]
impl From<ureq::Error> for ClientError {
    fn from(value: ureq::Error) -> Self {
        Self::Ureq(Box::new(value))
    }
}
#[cfg(feature = "async_client")]
impl From<reqwest::Error> for ClientError {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(value)
    }
}
impl From<StoreSerError> for ClientError {
    fn from(value: StoreSerError) -> Self {
        Self::Store(value)
    }
}
impl From<std::io::Error> for ClientError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}
impl From<InvalidStatusCode> for ClientError {
    fn from(value: InvalidStatusCode) -> Self {
        Self::InvalidStatusCode(value)
    }
}
impl From<serde_json::Error> for ClientError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}
impl From<ValueSerError> for ClientError {
    fn from(value: ValueSerError) -> Self {
        Self::Value(value)
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "sync_client")]
            Self::Ureq(u) => Some(u),
            Self::Store(s) => Some(s),
            Self::IO(e) => Some(e),
            Self::InvalidStatusCode(e) => Some(e),
            Self::SerdeJson(e) => Some(e),
            Self::Value(e) => Some(e),
            _ => None,
        }
    }
}
