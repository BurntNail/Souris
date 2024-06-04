#![cfg_attr(not(feature = "std"), no_std)]
// #![warn(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

extern crate alloc;
extern crate core;

#[cfg(feature = "std")]
extern crate std;

pub use chrono;
pub use hashbrown;
pub use serde_json;

pub mod store;
pub mod types;
pub mod utilities;
pub mod values;

#[cfg(feature = "axum")]
pub mod axum;

use crate::{store::StoreSerError, types::integer::IntegerSerError, values::ValueSerError};
use alloc::{
    format,
    string::{String, ToString},
};
use core::fmt::{Display, Formatter};

#[must_use]
pub fn display_bytes_as_hex_array(b: &[u8]) -> String {
    let mut out;
    match b.len() {
        0 => out = "[]".to_string(),
        1 => out = format!("[{:#X}]", b[0]),
        _ => {
            out = format!("[{:#X}", b[0]);
            for b in b.iter().skip(1) {
                out.push_str(&format!(", {b:#X}"));
            }
            out.push(']');
        }
    };
    out
}

#[derive(Debug)]
pub enum Error {
    Value(ValueSerError),
    Integer(IntegerSerError),
    Store(StoreSerError),
}

impl From<ValueSerError> for Error {
    fn from(value: ValueSerError) -> Self {
        Self::Value(value)
    }
}
impl From<IntegerSerError> for Error {
    fn from(value: IntegerSerError) -> Self {
        Self::Integer(value)
    }
}
impl From<StoreSerError> for Error {
    fn from(value: StoreSerError) -> Self {
        Self::Store(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Value(e) => write!(f, "Error with Value: {e}"),
            Self::Integer(e) => write!(f, "Error with Integer: {e}"),
            Self::Store(e) => write!(f, "Error with Store: {e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Value(e) => Some(e),
            Error::Integer(e) => Some(e),
            Error::Store(e) => Some(e),
        }
    }
}
