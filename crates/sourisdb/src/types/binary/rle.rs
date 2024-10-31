use itertools::Itertools;
use crate::types::binary::BinarySerError;
use crate::utilities::cursor::Cursor;
use alloc::vec;
use alloc::vec::Vec;

#[must_use]
pub fn rle(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.reverse();
    
    let mut output = vec![];
    
    let mut current_count = 1;
    let Some(mut current) = bytes.pop() else {
        return output;
    };

    while let Some(byte) = bytes.pop() {
        if current != byte {
            output.push(current_count);
            output.push(current);

            current_count = 0;
            current = byte;
        }
        current_count += 1;

        if current_count == u8::MAX {
            output.push(current_count);
            output.push(current);

            current_count = 0;
        }
    }
    if current_count != 0 {
        output.push(current_count);
        output.push(current);
    }

    output
}

///Uncompresses Run-Length-Encoded bytes
/// 
/// # Errors
/// - [`BinarySerError::NotEnoughBytes`] if there aren't enough bytes.
pub fn un_rle(len: usize, cursor: &mut Cursor<u8>) -> Result<Vec<u8>, BinarySerError> {
    //complicated version that's like 70% slower lol
    /*        let mut alloc_size = 0;
            let mut to_be_added = Vec::with_capacity(len);

            for _ in 0..len {
                let [count, byte] = cursor.read_exact().copied().ok_or(BinarySerError::NotEnoughBytes)?;
                alloc_size += count as usize;
                to_be_added.push((count, byte));
            }

            let mut output = Vec::with_capacity(alloc_size);

            for (count, byte) in to_be_added {
                (0..count).for_each(|_| output.push(byte));
            }
    */

    let mut output = vec![];
    for (count, byte) in cursor
        .read(len * 2)
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
    use crate::types::binary::{BinaryCompression, BinaryData};
    use crate::types::integer::Integer;
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

            let encoded = {
                let rle = rle(vec.clone());
                let mut out = Integer::usize(rle.len() / 2).ser().1;
                out.extend(&rle);
                out
            };

            let mut cursor = Cursor::new(&encoded);
            let BinaryData(decoded) =
                BinaryData::deser(BinaryCompression::RunLengthEncoding, &mut cursor).unwrap();

            assert_eq!(decoded, vec);
        }
    }
}
