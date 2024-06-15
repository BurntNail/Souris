//! `sourisdb` is a crate designed to provide a size-optimised way of transmitting a key-value store. There are a variety of methods used to achieve this goal ranging from variable-size integers to niche optimisations all detailed within [`values::Value`].
//!
//! The expected use-case is for web - in testing I've found this to be far more efficient than JSON whilst preserving type information AND providing additional types. Typically, SourisDB stores take around 25% less space than JSON objects even when minified.
//!
//! `sourisdb` can also be used for storage on-disk as it is entirely byte-order-agnostic as it deliberately stores everything using little-endian bytes.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

extern crate alloc;
extern crate core;

use alloc::{
    format,
    string::{String, ToString},
};
use core::fmt::Display;

pub use chrono;
pub use hashbrown;
pub use serde_json;

pub mod store;
pub mod types;
pub mod utilities;
pub mod values;

#[cfg(feature = "axum")]
pub mod axum;

#[cfg(feature = "client")]
pub mod client;

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
