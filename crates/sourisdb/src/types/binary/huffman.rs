use crate::utilities::bits::Bits;
use crate::utilities::cursor::Cursor;
use crate::utilities::huffman::{Huffman, HuffmanSerError};
use alloc::vec;
use alloc::vec::Vec;
use itertools::Itertools;
use crate::types::integer::{Integer, SignedState};

pub fn huffman (input: &[u8]) -> Vec<u8> {
    if input.is_empty() {
        return vec![0];
    } else if input.iter().all_equal() {
        let mut n = Integer::usize(input.len()).ser().1;
        n.insert(0, 1);
        n.push(input[0]);
        return n;
    }
    
    let Some((huffman, encoded)) = Huffman::new_and_encode(input.iter().copied()) else {
        unreachable!("checked for empty input already :)")
    };
    
    let serialised_huffman = huffman.ser();
    let serialised_bits = encoded.ser();
    
    let mut output = vec![2];
    output.extend(serialised_huffman);
    output.extend(serialised_bits);
    
    output
}

pub fn un_huffman (cursor: &mut Cursor<u8>) -> Result<Vec<u8>, HuffmanSerError> {
    let Some(first_byte) = cursor.next().copied() else {
        return Err(HuffmanSerError::NotEnoughBytes);
    };
    Ok(match first_byte {
        0 => vec![],
        1 => {
            let count: usize = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
            let element = cursor.next().copied().ok_or(HuffmanSerError::NotEnoughBytes)?;
            vec![element; count]
        },
        _ => {
            let huffman = Huffman::<u8>::deser(cursor)?;
            let bits = Bits::deser(cursor)?;

            huffman.decode(bits)?
        }
    })
    
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