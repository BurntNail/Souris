//! A collection of utilities for use with `sourisdb`.
//!
//! ## `cursor`
//! [`cursor::Cursor`] immutable view into a slice with a cursor head.
//!
//! ## `bits`
//! [`bits::Bits`] provides a way of storing individual bits, backed by a [`alloc::vec::Vec`] or [`u8`]s.
//!
//! ## `huffman`
//! [`huffman::Huffman`] is a huffman coder.

pub mod bits;
pub mod cursor;
pub mod huffman;
