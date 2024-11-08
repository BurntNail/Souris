use crate::{
    types::integer::{Integer, SignedState},
    utilities::{
        bits::Bits,
        cursor::Cursor,
        huffman::{Huffman, HuffmanSerError},
    },
};
use alloc::{vec, vec::Vec};
use itertools::Itertools;

pub fn huffman(input: &[u8]) -> Vec<u8> {
    if input.is_empty() {
        return vec![0];
    } else if input.iter().all_equal() {
        let mut n = Integer::usize(input.len()).ser().1;
        n.insert(0, 1);
        n.push(input[0]);
        return n;
    }

    let (huffman, encoded) =
        Huffman::new_and_encode(input.iter().copied()).expect("already checked for empty list");

    let serialised_huffman = huffman.ser();
    let serialised_bits = encoded.ser();

    let mut output = vec![2];
    output.extend(serialised_huffman);
    output.extend(serialised_bits);

    output
}

pub fn un_huffman(cursor: &mut Cursor<u8>) -> Result<Vec<u8>, HuffmanSerError> {
    let Some(first_byte) = cursor.next().copied() else {
        return Err(HuffmanSerError::NotEnoughBytes);
    };
    Ok(match first_byte {
        0 => vec![],
        1 => {
            let count: usize = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
            let element = cursor
                .next()
                .copied()
                .ok_or(HuffmanSerError::NotEnoughBytes)?;
            vec![element; count]
        }
        _ => {
            let huffman = Huffman::<u8>::deser(cursor)?;
            let bits = Bits::deser(cursor)?;

            huffman.decode(bits)?
        }
    })
}

#[cfg(test)]
mod tests {
    use super::super::CASES;
    use crate::types::binary::{
        huffman::{huffman, un_huffman},
        test_roundtrip,
    };
    use proptest::proptest;

    #[test]
    fn test_huff_specific_cases() {
        for case in CASES {
            test_roundtrip(case, huffman, un_huffman);
        }
    }

    proptest! {
        #[test]
        fn proptest_huffman_1 (v: [u8; 1]) {
            test_roundtrip(&v, huffman, un_huffman);
        }

        #[test]
        fn proptest_huffman_2 (v: [u8; 2]) {
            test_roundtrip(&v, huffman, un_huffman);
        }

        #[test]
        fn proptest_huffman_10 (v: [u8; 10]) {
            test_roundtrip(&v, huffman, un_huffman);
        }

        #[test]
        fn proptest_huffman_256 (v: [u8; 256]) {
            test_roundtrip(&v, huffman, un_huffman);
        }
    }
}
