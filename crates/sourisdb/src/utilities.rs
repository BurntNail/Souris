//! A collection of utilities for use with `sourisdb`.
//!
//! ## `cursor`
//! [`cursor::Cursor`] immutable view into a slice with a cursor head.
//!
//! ## `value_utils`
//! [`value_utils::get_value_from_stdin`] provides a convenient way to get a value from standard-in, utilising the [`dialoguer`] library.
//!
//! NB: to get access to `value_utils`, the `std` feature must be enabled.

mod bits;
pub mod cursor;
pub mod huffman;
#[cfg(feature = "std")]
pub mod value_utils;
