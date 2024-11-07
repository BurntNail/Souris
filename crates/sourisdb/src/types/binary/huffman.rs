use crate::utilities::cursor::Cursor;
use crate::utilities::huffman::{Huffman, HuffmanSerError};

pub fn huffman (input: &[u8]) -> Vec<u8> {
    let Some((huffman, encoded)) = Huffman::new_and_encode(input.iter().copied()) else {
        return vec![];
    };
    
    todo!()
}

pub fn un_huffman (cursor: &mut Cursor<u8>) -> Result<Vec<u8>, HuffmanSerError> {
    todo!()
}