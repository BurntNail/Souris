use crate::utilities::bits::Bits;
use crate::utilities::cursor::Cursor;
use crate::utilities::huffman::{Huffman, HuffmanSerError};
use alloc::vec;
use alloc::vec::Vec;

pub fn huffman (input: &[u8]) -> Vec<u8> {
    let Some((huffman, encoded)) = Huffman::new_and_encode(input.iter().copied()) else {
        return vec![];
    };
    
    let serialised_huffman = huffman.ser();
    let serialised_bits = encoded.ser();
    
    let mut output = serialised_huffman;
    output.extend(serialised_bits);
    
    output
}

pub fn un_huffman (cursor: &mut Cursor<u8>) -> Result<Vec<u8>, HuffmanSerError> {
    let huffman = Huffman::<u8>::deser(cursor)?;
    let bits = Bits::deser(cursor)?;

    huffman.decode(bits)
}

#[cfg(test)]
mod tests {
    use super::{super::CASES, *};
    use alloc::format;
    use proptest::{prop_assert_eq, proptest};
    use crate::types::binary::huffman::{huffman, un_huffman};

    #[test]
    fn test_huff_specific_cases() {
        for case in CASES {
            let vec = case.to_vec();

            let encoded = huffman(&vec);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_huffman(&mut cursor).unwrap();

            assert_eq!(decoded, vec);
        }
    }

    proptest! {
        #[test]
        fn proptest_huff_one (v: [u8; 1]) {
            let v = v.to_vec();

            let encoded = huffman(&v);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_huffman(&mut cursor).unwrap();

            prop_assert_eq!(v, decoded);
        }

        #[test]
        fn proptest_huff_two (v: [u8; 2]) {
            let v = v.to_vec();

            let encoded = huffman(&v);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_huffman(&mut cursor).unwrap();

            prop_assert_eq!(v, decoded);
        }

        #[test]
        fn proptest_huff_ten (v: [u8; 10]) {
            let v = v.to_vec();

            let encoded = huffman(&v);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_huffman(&mut cursor).unwrap();

            prop_assert_eq!(v, decoded);
        }
    }
}