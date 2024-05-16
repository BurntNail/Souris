use crate::{
    store::{Store, StoreSerError},
    utilities::cursor::Cursor,
    values::{Value, ValueSerError},
};
use alloc::{boxed::Box, format, string::String}; //boxed::Box is used for async_trait
use axum::{
    async_trait,
    body::Bytes,
    extract::{rejection::BytesRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};

impl IntoResponse for Value {
    fn into_response(self) -> Response {
        match self.ser() {
            Ok(b) => Bytes::from(b).into_response(),
            Err(e) => SourisRejection::Value(e, false).into_response(),
        }
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequest<S> for Value {
    type Rejection = SourisRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state).await?;
        let val = match Value::deser(&mut Cursor::new(&bytes)) {
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

#[non_exhaustive]
pub enum SourisRejection {
    CouldNotGetBytes(BytesRejection),
    Value(ValueSerError, bool),
    Store(StoreSerError, bool),
}
impl From<BytesRejection> for SourisRejection {
    fn from(value: BytesRejection) -> Self {
        Self::CouldNotGetBytes(value)
    }
}

impl SourisRejection {
    #[must_use]
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
    pub fn status(&self) -> StatusCode {
        match self {
            Self::CouldNotGetBytes(br) => br.status(),
            Self::Value(_, true) | Self::Store(_, true) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Value(_, false) | Self::Store(_, false) => StatusCode::BAD_REQUEST,
        }
    }
}

impl IntoResponse for SourisRejection {
    fn into_response(self) -> Response {
        (self.status(), self.body_text()).into_response()
    }
}
