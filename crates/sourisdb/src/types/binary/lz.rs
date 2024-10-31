use crate::types::binary::BinarySerError;
use crate::utilities::cursor::Cursor;
use alloc::vec::Vec;

#[must_use]
pub fn lz (bytes: Vec<u8>) -> Vec<u8> {
    core::mem::drop(bytes);
    todo!()
}

///Uncompresses LZ-format bytes
/// 
/// # Errors
/// - To be written!
pub fn un_lz (_len: usize, _cursor: &mut Cursor<u8>) -> Result<Vec<u8>, BinarySerError> {
    todo!()
}