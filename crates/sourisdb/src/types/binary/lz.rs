use crate::types::binary::BinarySerError;
use crate::utilities::cursor::Cursor;
use alloc::vec::Vec;
use alloc::vec;
use hashbrown::HashSet;
use crate::types::integer::{Integer, SignedState};

const SLIDING_WINDOW_SIZE: usize = 4096;

enum LzItem {
    Byte(u8),
    Token {
        offset: usize,
        length: usize
    }
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn lz (bytes: Vec<u8>) -> Vec<u8> {
    fn subslice_in_slice (subslice: &[u8], slice: &[u8]) -> Option<usize> {
        let subslice_len = subslice.len();
        let mut i = 0;
        let mut offset = 0;

        for element in slice.iter().copied() {
            if subslice_len <= offset {
                return Some(i - subslice_len);
            }

            if subslice[offset] == element {
                offset += 1;
            } else {
                offset = 0;
            }
            i += 1;
        }

        None
    }

    if bytes.is_empty() {
        return Integer::usize(0).ser().1;
    }

    let mut compressed = vec![];
    let mut search_buffer = vec![];
    let mut tmp = vec![];
    let mut i = 0;

    for byte in bytes.iter().copied() {
        tmp.push(byte);

        let found_index = subslice_in_slice(&tmp, &bytes);
        if found_index.is_none_or(|x| x == bytes.len() - 1) {
            if tmp.len() > 1 {
                //if we failed, get the previous index
                let length = tmp.len();
                let index = subslice_in_slice(&tmp[..(length - 1)], &bytes).unwrap(); //unreachable
                let offset = i - index - length;
                
                compressed.push(LzItem::Token {length, offset});
            } else {
                compressed.push(LzItem::Byte(byte));
            }
        }

        search_buffer.push(byte);
        i += 1;
        
        if tmp.len() > SLIDING_WINDOW_SIZE {
            tmp.remove(0);
        }
    }

    
    let token_indicies: Vec<usize> = compressed.iter().enumerate().filter_map(|(i, item)| {
        match item {
            LzItem::Byte(_) => None,
            LzItem::Token {length: _, offset: _} => Some(i),
        }
    }).collect();
    
    let mut output = Integer::usize(token_indicies.len()).ser().1;
    for index in token_indicies {
        output.extend(Integer::usize(index).ser().1);
    }
    
    output.extend(Integer::usize(compressed.len()).ser().1);
    
    for item in compressed {
        match item {
            LzItem::Byte(b) => output.push(b),
            LzItem::Token {offset, length} => {
                output.extend(Integer::usize(offset).ser().1);
                output.extend(Integer::usize(length).ser().1);
            }
        }
    }
    
    output
}

///Uncompresses LZ-format bytes
pub fn un_lz (cursor: &mut Cursor<u8>) -> Result<Vec<u8>, BinarySerError> {
    let number_of_replacements: usize = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
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
            
            output.extend((&output[start..end]).to_vec());
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

            let encoded = lz(vec.clone());
            let mut cursor = Cursor::new(&encoded);
            let decoded = un_lz(&mut cursor).unwrap();

            assert_eq!(decoded, vec);
        }
    }
}