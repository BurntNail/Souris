use lz4_flex::{compress, decompress};
use crate::types::binary::BinarySerError;
use crate::types::integer::{Integer, SignedState};
use crate::utilities::cursor::Cursor;
use alloc::vec;
use alloc::vec::Vec;

#[must_use]
pub fn lz (input: &[u8]) -> Vec<u8> {
    let size = Integer::usize(input.len()).ser().1;
    if input.is_empty() {
        return size;
    }
    
    let compressed = compress(input);
    let mut output = size; //size of input
    output.extend(Integer::usize(compressed.len()).ser().1); //size of compressed
    output.extend(compressed); //compressed
    
    output
}

///Decompresses LZ compressed data
/// 
/// # Errors
/// - [`IntegerSerError`] if we cannot deserialise an integer
/// - [`BinarySerError::NotEnoughBytes`] if there aren't enough bytes
/// - [`DecompressError`] if we fail to decompress the bytes
pub fn un_lz (cursor: &mut Cursor<u8>) -> Result<Vec<u8>, BinarySerError> {
    let input_len: usize = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
    if input_len == 0 {
        return Ok(vec![]);
    }
    
    let compressed_len = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
    let compressed = cursor.read(compressed_len).ok_or(BinarySerError::NotEnoughBytes)?;
    
    Ok(decompress(compressed, input_len)?)
}

//leftover from when I tried to actually implement lz, but was very slow
//I used this: //LZ77, not LZSS from https://go-compression.github.io/algorithms/lz/
/*#[derive(Copy, Clone, Debug)]
const MAX_SLIDING_WINDOW_SIZE: usize = 128;

enum LzItem {
    Byte(u8),
    Token { offset: usize, length: usize },
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn lz(bytes: &[u8]) -> Vec<u8> {
    let mut locations_found: HashMap<_, HashMap<_, _>> = HashMap::new();
    let mut subslice_in_slice = |check: (usize, usize), search: usize| -> Option<usize> {
        let so_far = locations_found.entry(search).or_default();
        let index = so_far.entry(check).or_insert_with(|| {
            memchr::memmem::find(&bytes[..search], &bytes[check.0..check.1])
        });
        *index
    };

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
    let mut search_end = 0;


    for i in 0..bytes.len() {
        let is_last = i == bytes.len() - 1;
        let next_index = subslice_in_slice(
            (check_start, check_end + 1),
            search_end
        );

        let mut char_is_added = false;
        if next_index.is_none() || is_last {
            if is_last && next_index.is_some() {
                char_is_added = true;
                check_end += 1;
            }

            if (check_end - check_start) > 1 {
                let Some(index) = subslice_in_slice(
                    (check_start, check_end),
                    search_end,
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

            //has to re-alloc as otherwise mutable shenanigans
            output.extend(output[start..end].to_vec());
        } else {
            output.push(*cursor.next().ok_or(BinarySerError::NotEnoughBytes)?);
        }
    }

    Ok(output)
}
*/

#[cfg(test)]
mod tests {
    use proptest::{prop_assert_eq, proptest};
    use super::*;
    use super::super::CASES;
    use alloc::format;

    #[test]
    fn test_lz_specific_cases() {
        for case in CASES {
            let vec = case.to_vec();

            let encoded = lz(&vec);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_lz(&mut cursor).unwrap();

            assert_eq!(decoded, vec);
        }
    }
    
    proptest!{
        #[test]
        fn proptest_lz_one (v: [u8; 1]) {
            let v = v.to_vec();
            
            let encoded = lz(&v);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_lz(&mut cursor).unwrap();
            
            prop_assert_eq!(v, decoded);
        }
        
        #[test]
        fn proptest_lz_two (v: [u8; 2]) {
            let v = v.to_vec();
            
            let encoded = lz(&v);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_lz(&mut cursor).unwrap();
            
            prop_assert_eq!(v, decoded);
        }
        
        #[test]
        fn proptest_lz_ten (v: [u8; 10]) {
            let v = v.to_vec();
            
            let encoded = lz(&v);
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_lz(&mut cursor).unwrap();
            
            prop_assert_eq!(v, decoded);
        }
    }
}