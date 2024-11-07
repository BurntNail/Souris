use crate::{
    types::{
        binary::BinarySerError,
        integer::{Integer, SignedState},
    },
    utilities::cursor::Cursor,
};
use alloc::{vec, vec::Vec};
use itertools::Itertools;

#[must_use]
pub fn rle(bytes: &[u8]) -> Vec<u8> {
    let mut iter = bytes.iter();

    match iter.next().copied() {
        None => Integer::usize(0).ser().1,
        Some(mut current) => {
            let mut compressed = vec![];
            let mut current_count = 1;

            for byte in iter.copied() {
                if current == byte && current_count < u8::MAX {
                    current_count += 1;
                } else {
                    compressed.push(current_count);
                    compressed.push(current);
                    current = byte;
                    current_count = 1;
                }
            }
            if current_count != 0 {
                compressed.push(current_count);
                compressed.push(current);
            }

            let mut output = Integer::usize(compressed.len()).ser().1;
            output.extend(&compressed);

            output
        }
    }
}

///Uncompresses Run-Length-Encoded bytes
///
/// # Errors
/// - [`BinarySerError::NotEnoughBytes`] if there aren't enough bytes.
pub fn un_rle(cursor: &mut Cursor<u8>) -> Result<Vec<u8>, BinarySerError> {
    let len: usize = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
    if len == 0 {
        return Ok(vec![]);
    }

    let mut output = vec![];

    for (count, byte) in cursor
        .read(len)
        .ok_or(BinarySerError::NotEnoughBytes)?
        .iter()
        .copied()
        .tuples()
    {
        (0..count).for_each(|_| output.push(byte));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use proptest::{prop_assert_eq, proptest};
    use super::{super::CASES, *};
    use alloc::format;

    #[test]
    fn test_rle_specific_cases() {
        for case in CASES {
            let vec = case.to_vec();

            let encoded = rle(&vec);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_rle(&mut cursor).unwrap();

            assert_eq!(decoded, vec);
        }
    }

    proptest! {
        #[test]
        fn proptest_rle_one (v: [u8; 1]) {
            let v = v.to_vec();

            let encoded = rle(&v);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_rle(&mut cursor).unwrap();

            prop_assert_eq!(v, decoded);
        }

        #[test]
        fn proptest_rle_two (v: [u8; 2]) {
            let v = v.to_vec();

            let encoded = rle(&v);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_rle(&mut cursor).unwrap();

            prop_assert_eq!(v, decoded);
        }

        #[test]
        fn proptest_rle_ten (v: [u8; 10]) {
            let v = v.to_vec();

            let encoded = rle(&v);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_rle(&mut cursor).unwrap();

            prop_assert_eq!(v, decoded);
        }
    }
}
