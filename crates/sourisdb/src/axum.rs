//!This module provides utilities for use with [`axum`] and requires the `axum` feature to be enabled.
//!
//! The utilities consist of `IntoResponse` and `FromRequest` extractors & responses for use in async handlers. Both are wrappers over [`axum::body::Bytes`] with extra logic for serialisation.
//!
//! ## Extractor Example
//! ```rust
//! use axum::{extract::State, http::StatusCode};
//! use sourisdb::{types::integer::Integer, values::Value};
//!
//! #[axum::debug_handler]
//! async fn fn_add_if_number (State(mut state): State<Vec<Integer>>, value: Value) -> StatusCode {
//!    match value.to_int() {
//!        Some(i) => {
//!            state.push(i);
//!            StatusCode::OK
//!        }
//!        None => StatusCode::BAD_REQUEST
//!    }
//! }
//! ```
//!
//! ## Response Example
//!```rust
//! use axum::extract::State;
//! use sourisdb::{store::Store, types::integer::Integer, values::Value};
//!
//! #[axum::debug_handler]
//! async fn get_numbers(State(state): State<Vec<Integer>>) -> Store {
//!    let mut store = Store::default();
//!    store.insert("numbers".to_string(), Value::Array(state.into_iter().map(Value::Integer).collect()));
//!    store
//! }
//! ```

use alloc::{format, string::String};
use core::fmt::{Display, Formatter};

use axum::{
    async_trait,
    body::Bytes,
    extract::{rejection::BytesRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    store::{Store, StoreSerError},
    utilities::cursor::Cursor,
    values::{Value, ValueSerError},
};

//boxed::Box is used for async_trait

impl IntoResponse for Value {
    fn into_response(self) -> Response {
        //TODO: huffman ser
        let b = self.ser(None);
        Bytes::from(b).into_response()
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequest<S> for Value {
    type Rejection = SourisRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state).await?;
        let val = match Value::deser(&mut Cursor::new(&bytes), None) {
            Ok(v) => v,
            Err(e) => return Err(SourisRejection::Value(e, true)),
        };
        Ok(val)
    }
}

impl IntoResponse for Store {
    fn into_response(self) -> Response {
        match self.ser() {
            Ok(b) => Bytes::from(b).into_response(),
            Err(e) => SourisRejection::Store(e, true).into_response(),
        }
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequest<S> for Store {
    type Rejection = SourisRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state).await?;
        let val = match Store::deser(bytes.as_ref()) {
            Ok(v) => v,
            Err(e) => return Err(SourisRejection::Store(e, true)),
        };
        Ok(val)
    }
}

///Error struct for if there is a failure de/ser-ing a `Store` using `FromRequest` or `IntoResponse`
#[non_exhaustive]
pub enum SourisRejection {
    ///signifies that there was an error from [`axum::body::Bytes`] getting the bytes
    CouldNotGetBytes(BytesRejection),
    ///signifies that there was an error with the [`Value`] - the boolean signifies whether it was serialising or deserialising (`true` is serialising)
    Value(ValueSerError, bool),
    ///signifies that there was an error with the [`Store`] - the boolean signifies whether it was serialising or deserialising (`true` is serialising)
    Store(StoreSerError, bool),
}
impl From<BytesRejection> for SourisRejection {
    fn from(value: BytesRejection) -> Self {
        Self::CouldNotGetBytes(value)
    }
}

impl SourisRejection {
    #[must_use]
    ///Provides simple error descriptions for use in a response and is used for the [`Display`] implementation.
    pub fn body_text(&self) -> String {
        match self {
            Self::CouldNotGetBytes(br) => format!("Could not get bytes: {br}"),
            Self::Value(e, was_ser) => {
                let ser = if *was_ser { "serialise" } else { "deserialise" };
                format!("Could not {ser} value: {e}")
            }
            Self::Store(e, was_ser) => {
                let ser = if *was_ser { "serialise" } else { "deserialise" };
                format!("Could not {ser} store: {e}")
            }
        }
    }

    #[must_use]
    ///Converts each kind of rejection into an error code:
    ///- [`Self::CouldNotGetBytes`]: defer to [`BytesRejection::status`]
    ///- [`Self::Value`] or [`Self::Store`] if deserialising: assume the request was the error and return [`StatusCode::BAD_REQUEST`]
    ///- [`Self::Value`] or [`Self::Store`] if serialising: assume the error was internal and return [`StatusCode::INTERNAL_SERVER_ERROR`]
    pub fn status(&self) -> StatusCode {
        match self {
            Self::CouldNotGetBytes(br) => br.status(),
            Self::Value(_, true) | Self::Store(_, true) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Value(_, false) | Self::Store(_, false) => StatusCode::BAD_REQUEST,
        }
    }
}

impl Display for SourisRejection {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.body_text())
    }
}

impl IntoResponse for SourisRejection {
    fn into_response(self) -> Response {
        (self.status(), self.body_text()).into_response()
    }
}
