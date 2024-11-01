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
pub fn rle(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.reverse();

    let mut current_count = 1;
    let Some(mut current) = bytes.pop() else {
        return Integer::usize(0).ser().1;
    };

    let mut compressed = vec![];
    while let Some(byte) = bytes.pop() {
        if current != byte {
            compressed.push(current_count);
            compressed.push(current);

            current_count = 0;
            current = byte;
        }
        current_count += 1;

        if current_count == u8::MAX {
            compressed.push(current_count);
            compressed.push(current);

            current_count = 0;
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
    use super::*;

    #[test]
    fn test_rle_specific_cases() {
        const CASES: &[&[u8]] = &[
            &[
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF,
            ],
            &[],
            &[0],
            &[0x12, 0x12, 0x12, 0xDE, 0xAD, 0xBE, 0xEF],
        ];

        for case in CASES {
            let vec = case.to_vec();

            let encoded = rle(vec.clone());
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_rle(&mut cursor).unwrap();

            assert_eq!(decoded, vec);
        }
    }
}
