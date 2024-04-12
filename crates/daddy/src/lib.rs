#![no_std]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

extern crate alloc;

pub mod niches;
pub mod store;
pub mod utilities;
pub mod values;
pub mod version;

//TODO: explore ways of making this work `no_std`
//TODO: tests
//TODO: docs
