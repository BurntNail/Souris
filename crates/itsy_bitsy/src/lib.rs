#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

extern crate alloc;

pub mod store;
pub mod types;
pub mod utilities;
pub mod values;
pub mod version;
