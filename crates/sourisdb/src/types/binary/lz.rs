use crate::{
    types::{
        binary::BinarySerError,
        integer::{Integer, SignedState},
    },
    utilities::cursor::Cursor,
};
use alloc::{vec, vec::Vec};
use hashbrown::HashSet;

const MAX_SLIDING_WINDOW_SIZE: usize = 4096;

#[derive(Copy, Clone, Debug)]
enum LzItem {
    Byte(u8),
    Token { offset: usize, length: usize },
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn lz(bytes: &[u8]) -> Vec<u8> {
    fn subslice_in_slice(subslice: &[u8], slice: &[u8]) -> Option<usize> {
        if subslice.is_empty() || slice.len() < subslice.len() {
            return None;
        }
        slice
            .windows(subslice.len())
            .position(|window| window == subslice)
    }

    if bytes.is_empty() {
        //number of replacements:
        let mut output = Integer::usize(0).ser().1;
        //number of items
        let number_of_items = output.clone();
        output.extend(number_of_items);

        return output;
    }

    let mut compressed = vec![];

    let mut check_start = 0;
    let mut check_end = 0;
    let search_start = 0;
    let mut search_end = 0;

    for i in 0..bytes.len() {
        let is_last = i == bytes.len() - 1;
        let next_index = subslice_in_slice(
            &bytes[check_start..=check_end],
            &bytes[search_start..search_end],
        );

        let mut char_is_added = false;
        if next_index.is_none() || is_last {
            if is_last && next_index.is_some() {
                char_is_added = true;
                check_end += 1;
            }

            if (check_end - check_start) > 1 {
                let Some(index) = subslice_in_slice(
                    &bytes[check_start..check_end],
                    &bytes[search_start..search_end],
                ) else {
                    unreachable!("Invalid temporary buffer in LZ77 with multiple failing elements");
                };
                let length = check_end - check_start;
                let offset = i - index - length;

                //TODO: LZSS rather than LZ77

                compressed.push(LzItem::Token { offset, length });
                search_end = check_end;
            } else {
                compressed.extend(
                    bytes[check_start..check_end]
                        .iter()
                        .copied()
                        .map(LzItem::Byte),
                );
                search_end = check_end;
            }

            check_start = check_end;
        }

        if !char_is_added {
            check_end += 1;
        }

        if (check_end - check_start) > MAX_SLIDING_WINDOW_SIZE {
            check_start += 1;
        }
    }

    compressed.extend(
        bytes[check_start..check_end]
            .iter()
            .copied()
            .map(LzItem::Byte),
    );

    let token_indicies: Vec<usize> = compressed
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match item {
            LzItem::Byte(_) => None,
            LzItem::Token {
                length: _,
                offset: _,
            } => Some(i),
        })
        .collect();

    let mut output = Integer::usize(token_indicies.len()).ser().1;
    for index in token_indicies {
        output.extend(Integer::usize(index).ser().1);
    }

    output.extend(Integer::usize(compressed.len()).ser().1);

    for item in compressed {
        match item {
            LzItem::Byte(b) => output.push(b),
            LzItem::Token { offset, length } => {
                output.extend(Integer::usize(offset).ser().1);
                output.extend(Integer::usize(length).ser().1);
            }
        }
    }

    output
}

///Uncompresses LZ-format bytes
///
/// # Errors
/// - [`IntegerSerError`] if we cannot deserialise one of the component [`Integer`]s
/// - [`BinarySerError::NotEnoughBytes`] if there aren't enough bytes.
pub fn un_lz(cursor: &mut Cursor<u8>) -> Result<Vec<u8>, BinarySerError> {
    let number_of_replacements: usize =
        Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
    let mut token_indicies = HashSet::new();
    for _ in 0..number_of_replacements {
        let index: usize = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
        token_indicies.insert(index);
    }

    let number_of_items: usize = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
    let mut output = vec![];

    for i in 0..number_of_items {
        if token_indicies.contains(&i) {
            let offset: usize = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
            let length: usize = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;

            let start = output.len() - offset;
            let end = start + length;

            output.extend(output[start..end].to_vec());
        } else {
            output.push(*cursor.next().ok_or(BinarySerError::NotEnoughBytes)?);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lz_specific_cases() {
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

            let encoded = lz(&vec);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_lz(&mut cursor).unwrap();

            assert_eq!(decoded, vec);
        }
    }
}
