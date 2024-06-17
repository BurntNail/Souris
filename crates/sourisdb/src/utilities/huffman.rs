use alloc::{boxed::Box, string::String, vec::Vec};
use core::fmt::{Display, Formatter};

use hashbrown::HashMap;

use crate::{
    display_bytes_as_hex_array,
    types::integer::{Integer, IntegerSerError, SignedState},
    utilities::{bits::Bits, cursor::Cursor},
};

#[derive(Debug, Clone)]
pub struct Huffman {
    chars_to_bits: HashMap<char, Bits>,
    bits_to_chars: HashMap<Bits, char>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum Node {
    Leaf(char),
    Branch { left: Box<Node>, right: Box<Node> },
}

#[derive(Debug)]
pub enum HuffmanSerError {
    InvalidCharacter(u32),
    Integer(IntegerSerError),
}

impl From<IntegerSerError> for HuffmanSerError {
    fn from(value: IntegerSerError) -> Self {
        Self::Integer(value)
    }
}

impl Display for HuffmanSerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            HuffmanSerError::InvalidCharacter(bytes) => write!(
                f,
                "Found invalid character: {}",
                display_bytes_as_hex_array(&bytes.to_le_bytes())
            ),
            HuffmanSerError::Integer(i) => write!(f, "Error with integer: {i}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for HuffmanSerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Integer(i) => Some(i),
            Self::InvalidCharacter(_) => None,
        }
    }
}

struct MinHeap<T> {
    list: Vec<(T, usize)>,
}

impl<T> MinHeap<T> {
    pub fn new(list: Vec<(T, usize)>) -> Self {
        Self { list }
    }

    pub fn push(&mut self, item: T, weight: usize) {
        self.list.push((item, weight));
    }
}

impl<T> Iterator for MinHeap<T> {
    type Item = (T, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self
            .list
            .iter()
            .enumerate()
            .min_by_key(|(_i, (_t, weight))| *weight)?
            .0;
        Some(self.list.swap_remove(index))
    }
}

impl Huffman {
    fn to_node_tree(data: &str) -> Option<Node> {
        if data.len() < 2 {
            return None;
        }

        let mut frequency_table: HashMap<Node, usize> = HashMap::new();
        for ch in data.chars() {
            *frequency_table.entry(Node::Leaf(ch)).or_default() += 1_usize;
        }
        let mut min_heap: MinHeap<Node> = MinHeap::new(frequency_table.into_iter().collect());

        loop {
            let (least_frequent_ch, least_frequent_weight) = min_heap.next().unwrap(); //checked for len earlier
            let Some((next_least_frequent_ch, next_least_frequent_weight)) = min_heap.next() else {
                return Some(least_frequent_ch);
            };

            let new_node = Node::Branch {
                left: Box::new(least_frequent_ch),
                right: Box::new(next_least_frequent_ch),
            };
            let weight = least_frequent_weight + next_least_frequent_weight;
            min_heap.push(new_node, weight);
        }
    }

    #[must_use]
    pub fn new(data: impl AsRef<str>) -> Option<Self> {
        fn add_node_to_table(
            node: Node,
            chars_to_bits: &mut HashMap<char, Bits>,
            bits_to_chars: &mut HashMap<Bits, char>,
            bits_so_far: Bits,
        ) {
            match node {
                Node::Leaf(ch) => {
                    chars_to_bits.insert(ch, bits_so_far.clone());
                    bits_to_chars.insert(bits_so_far, ch);
                }
                Node::Branch { left, right } => {
                    let mut left_bits = bits_so_far.clone();
                    let mut right_bits = bits_so_far.clone();
                    left_bits.push(false);
                    right_bits.push(true);

                    add_node_to_table(*left, chars_to_bits, bits_to_chars, left_bits);
                    add_node_to_table(*right, chars_to_bits, bits_to_chars, right_bits);
                }
            }
        }

        let mut chars_to_bits = HashMap::new();
        let mut bits_to_chars = HashMap::new();

        let nodes = Self::to_node_tree(data.as_ref())?;

        add_node_to_table(
            nodes,
            &mut chars_to_bits,
            &mut bits_to_chars,
            Bits::default(),
        );

        Some(Self {
            chars_to_bits,
            bits_to_chars,
        })
    }

    pub fn encode_string(&self, str: impl AsRef<str>) -> Option<Bits> {
        let mut bits = Bits::default();

        for ch in str.as_ref().chars() {
            let bits_to_add = self.chars_to_bits.get(&ch).cloned()?;
            bits.push_many(bits_to_add);
        }

        Some(bits)
    }

    #[must_use]
    pub fn decode_string(&self, bits: &Bits) -> Option<String> {
        let mut string = String::new();

        let mut accumulator = Bits::default();
        for i in 0..bits.len() {
            accumulator.push(bits[i]);

            if let Some(ch) = self.bits_to_chars.get(&accumulator).copied() {
                accumulator.clear();
                string.push(ch);
            }
        }

        if accumulator.is_empty() {
            Some(string)
        } else {
            None
        }
    }

    #[must_use]
    pub fn ser(&self) -> Vec<u8> {
        let len = self.bits_to_chars.len();
        let (_, mut bytes) = Integer::usize(len).ser();

        for (bits, ch) in self.bits_to_chars.clone() {
            let (_, ch) = Integer::u32(ch as u32).ser();
            bytes.extend(ch);
            bytes.extend(bits.ser());
        }

        bytes
    }

    pub fn deser(bytes: &mut Cursor<u8>) -> Result<Self, HuffmanSerError> {
        let len: usize = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;
        let mut bits_to_chars = HashMap::new();
        let mut chars_to_bits = HashMap::new();

        for _ in 0..len {
            let ch_bits = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;
            let Some(ch) = char::from_u32(ch_bits) else {
                return Err(HuffmanSerError::InvalidCharacter(ch_bits));
            };
            let bits = Bits::deser(bytes)?;

            bits_to_chars.insert(bits.clone(), ch);
            chars_to_bits.insert(ch, bits);
        }

        Ok(Self {
            chars_to_bits,
            bits_to_chars,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use proptest::{prop_assert_eq, proptest};

    use crate::utilities::huffman::{Huffman, Node};

    #[test]
    fn nodes_from_empty_string() {
        let huffman = Huffman::to_node_tree("");
        assert!(huffman.is_none());
    }

    #[test]
    fn nodes_from_one_char_repeated() {
        let huffman = Huffman::to_node_tree("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        let Node::Leaf(ch) = huffman else {
            panic!("didn't find leaf node at root");
        };
        assert_eq!(ch, 'a');
    }

    #[test]
    fn test_encode_decode_five_characters() {
        let data = "abcdeabcdabcabaaaaaa";
        let huffman = Huffman::new(data).unwrap();

        let encoded = huffman.encode_string(data).unwrap();
        let decoded = huffman.decode_string(&encoded).unwrap();

        assert_eq!(data, decoded);
    }

    proptest! {
        #[test]
        fn doesnt_crash (s in "\\PC*") {
            let _ = Huffman::new(s);
        }

        #[test]
        fn works_on_arbritrary_ascii_strings (s in " [a-zA-Z0-9]+") { //chuck a space in front to make sure there's two different characters
            let huffman = Huffman::new(&s).expect("unable to get huffman");

            let encoded = huffman.encode_string(&s).expect("unable to encode");
            let decoded = huffman.decode_string(&encoded).expect("unable to decode");

            prop_assert_eq!(s, decoded);
        }
    }
}
