//! A module that provides one sync and one async client for use with the `sourisd` API.
//!
//! To enable the sync client, use the `sync_client` feature, and for the async client add `async_client`. Both can be enabled without any issues.
//!
//! The methods available on both clients are identical, save the async ones being async. The [`ClientError`] type changes based off which features are enabled to hold the error types for the HTTP library.
//!
//! The sync client is backed by [`ureq`] and the async client by [`reqwest`].

use crate::{store::StoreSerError, values::ValueSerError};
use core::fmt::{Display, Formatter};
use http::StatusCode;
use serde::{Deserialize, Serialize};
#[cfg(feature = "async_client")]
pub use async_client::AsyncClient;
#[cfg(feature = "sync_client")]
pub use sync_client::SyncClient;

#[cfg(feature = "async_client")]
mod async_client;
#[cfg(feature = "sync_client")]
mod sync_client;

const fn bool_to_string (b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub enum CreationResult {
    InsertedKeyIntoExistingDB,
    OverwroteKeyInExistingDB,
    FoundExistingKey,
    InsertedKeyIntoNewDB,
    UnableToFindDB,
}

impl Display for CreationResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let txt = match self {
            Self::InsertedKeyIntoExistingDB => "Added new key to existing database",
            Self::OverwroteKeyInExistingDB => "Overwrote existing key in existing database",
            Self::FoundExistingKey => "Found existing key in existing database, didn't overwrite",
            Self::InsertedKeyIntoNewDB => "Added new key to new database",
            Self::UnableToFindDB => "Unable to find database"
        };
        write!(f, "{txt}")
    }
}

pub const DEFAULT_SOURISD_PORT: u32 = 7687;

///An error which could occur using one of the `sourisd` clients.
#[derive(Debug)]
pub enum ClientError {
    ///An error from `ureq` - this can only be a transport issue as HTTP error codes are handled in a separate variant - [`ClientError::HttpErrorCode`].
    #[cfg(feature = "sync_client")]
    Ureq(ureq::Transport),
    ///An error from `reqwest` - this could be from a variety of sources, but not HTTP error codes - thy are handled in [`ClientError::HttpErrorCode`].
    #[cfg(feature = "async_client")]
    Reqwest(reqwest::Error),
    ///An error de/ser-ialising a [`crate::store::Store`].
    Store(StoreSerError),
    ///An error de/ser-ialising a [`crate::values::Value`].
    Value(ValueSerError),
    ///A request was sent and a non 2xx code was returned.
    HttpErrorCode(StatusCode),
    ///An IO Error occured - this error variant occurs when reading in the body of the sync client.
    #[cfg(feature = "sync_client")]
    IO(std::io::Error),
    ///An invalid status code was found - this error occurs when turning a `u32` into a `StatusCode` in the sync client.
    #[cfg(feature = "sync_client")]
    InvalidStatusCode(http::status::InvalidStatusCode),
    ///In the clients' constructors, a request is made to the healthcheck endpoint of the server. This error occurs if that does not return `200 OK`.
    ServerNotHealthy(StatusCode),
    ///An error occurred with `serde_json`.
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
            #[cfg(feature = "sync_client")]
            Self::IO(e) => write!(f, "IO Error: {e}"),
            #[cfg(feature = "sync_client")]
            Self::InvalidStatusCode(e) => write!(f, "Invalid status code provided: {e}"),
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
impl From<ureq::Transport> for ClientError {
    fn from(value: ureq::Transport) -> Self {
        Self::Ureq(value)
    }
}
#[cfg(feature = "sync_client")]
impl From<ureq::Error> for ClientError {
    fn from(value: ureq::Error) -> Self {
        match value {
            ureq::Error::Status(status, _response) => match StatusCode::try_from(status) {
                Ok(sc) => ClientError::HttpErrorCode(sc),
                Err(e) => ClientError::InvalidStatusCode(e),
            },
            ureq::Error::Transport(transport_error) => ClientError::Ureq(transport_error),
        }
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
#[cfg(feature = "sync_client")]
impl From<std::io::Error> for ClientError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}
#[cfg(feature = "sync_client")]
impl From<http::status::InvalidStatusCode> for ClientError {
    fn from(value: http::status::InvalidStatusCode) -> Self {
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

#[cfg(feature = "std")]
impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "sync_client")]
            Self::Ureq(u) => Some(u),
            Self::Store(s) => Some(s),
            #[cfg(feature = "sync_client")]
            Self::IO(e) => Some(e),
            #[cfg(feature = "sync_client")]
            Self::InvalidStatusCode(e) => Some(e),
            Self::SerdeJson(e) => Some(e),
            Self::Value(e) => Some(e),
            _ => None,
        }
    }
}
