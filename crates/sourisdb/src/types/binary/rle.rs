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
    use super::{super::CASES, *};
    use crate::types::binary::test_roundtrip;
    use proptest::proptest;

    #[test]
    fn test_rle_specific_cases() {
        for case in CASES {
            test_roundtrip(case, rle, un_rle);
        }
    }

    proptest! {
        #[test]
        fn proptest_rle_1 (v: [u8; 1]) {
            test_roundtrip(&v, rle, un_rle);
        }

        #[test]
        fn proptest_rle_2 (v: [u8; 2]) {
            test_roundtrip(&v, rle, un_rle);
        }

        #[test]
        fn proptest_rle_10 (v: [u8; 10]) {
            test_roundtrip(&v, rle, un_rle);
        }

        #[test]
        fn proptest_rle_256 (v: [u8; 256]) {
            test_roundtrip(&v, rle, un_rle);
        }
    }
}
