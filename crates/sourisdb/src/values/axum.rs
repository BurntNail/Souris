use crate::{
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
            Err(e) => ValueRejection::Value(e).into_response(),
        }
    }
}

#[non_exhaustive]
pub enum ValueRejection {
    CouldNotGetBytes(BytesRejection),
    Value(ValueSerError),
}

impl From<BytesRejection> for ValueRejection {
    fn from(value: BytesRejection) -> Self {
        Self::CouldNotGetBytes(value)
    }
}
impl From<ValueSerError> for ValueRejection {
    fn from(value: ValueSerError) -> Self {
        Self::Value(value)
    }
}

impl ValueRejection {
    #[must_use]
    pub fn body_text(&self) -> String {
        match self {
            Self::CouldNotGetBytes(br) => format!("Could not get bytes: {br}"),
            Self::Value(e) => format!("Could not de/serialise value: {e}"),
        }
    }

    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self {
            Self::CouldNotGetBytes(br) => br.status(),
            Self::Value(_) => StatusCode::BAD_REQUEST,
        }
    }
}

impl IntoResponse for ValueRejection {
    fn into_response(self) -> Response {
        (self.status(), self.body_text()).into_response()
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequest<S> for Value {
    type Rejection = ValueRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state).await?;
        let val = Value::deser(&mut Cursor::new(&bytes))?;
        Ok(val)
    }
}
