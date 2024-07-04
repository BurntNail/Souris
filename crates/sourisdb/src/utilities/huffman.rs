//! A struct to provide [huffman](https://en.wikipedia.org/wiki/Huffman_coding)-encoding capabilities.
//!
//! It will happily operate on any `T`, assuming it implements [`Hash`], [`Eq`] and [`Clone`]:
//! - [`Hash`] & [`Eq`] are needed for the [`HashMap`]s which back the [`Huffman`] struct.
//! - [`Clone`] is needed as there are two [`HashMap`]s - one for converting each way.
//!
//! To construct it, simply provide it with an iterator of all the permutations of the data type you expect to see - repeats are helpful as this allows the huffman encoder to better optimise the tree.
//! ```rust
//! use sourisdb::utilities::bits::Bits;
//! use sourisdb::utilities::cursor::Cursor;
//! use sourisdb::utilities::huffman::Huffman;
//!
//! let deadpool_scores = [8_u8, 8, 9, 9, 8, 9, 7];
//! let marvels_scores = [3_u8, 4, 5, 2];
//! let marmite_scores = [1_u8, 10, 2, 8, 9];
//!
//! let combined_scores = deadpool_scores.into_iter().chain(marvels_scores.into_iter()).chain(marmite_scores.into_iter());
//! let scores_huffman: Huffman<u8> = Huffman::new(combined_scores).unwrap(); //scores for a variety of films
//!
//! let deadpool_bits = scores_huffman.encode(deadpool_scores.into_iter()).unwrap();
//! let encoded = deadpool_bits.ser();
//! assert!(encoded.len() < deadpool_scores.len()); //huffman can be more efficient than even u8s when storing common variants. Here, the encoded is stored using only 3 bytes!
//!
//! /* save the encoded to a file, then read them back */
//! let decoded = encoded;
//! let deadpool_recovered_bits = Bits::deser(&mut Cursor::new(&decoded)).unwrap();
//! let recovered_deadpool = scores_huffman.decode(deadpool_recovered_bits).unwrap();
//! assert_eq!(deadpool_scores.to_vec(), recovered_deadpool);
//! ```
//!
//! As you can see, huffman encoding preserves the order as well as the values in a lossless format. However, because the dictionary must also be encoded, this is best suited to longer collections.
//!
//! There are also extra functions for use with [`String`]s and [`char`]s as they are the original case this implementation was programmed for:
//! ```rust
//!use sourisdb::utilities::huffman::Huffman;
//!
//! let huffman: Huffman<char> = Huffman::new_str(r#"
//! According to all known laws
//! of aviation,
//! there is no way a bee
//! should be able to fly.
//! Its wings are too small to get
//! its fat little body off the ground.
//! The bee, of course, flies anyway
//! because bees don't care
//! what humans think is impossible.
//! Yellow, black. Yellow, black.
//! Yellow, black. Yellow, black.
//! Ooh, black and yellow!
//! Let's shake it up a little.
//! "#).unwrap();
//!
//! let input = "bees do be flying";
//! let bits = huffman.encode_string(input).unwrap();
//! let output = huffman.decode_string(bits).unwrap();
//! assert_eq!(input, &output);
//! ```
//!
//! If you don't already have a sample corpus of English, there is a preset for the [Reuters21578 corpus](http://www.daviddlewis.com/resources/testcollections/reuters21578/):
//!```rust
//! use sourisdb::utilities::huffman::Huffman;
//! let huffman = Huffman::new_with_english_frequencies();
//!
//! let input = "The quick brown fox jumps over the lazy dog.";
//! let bits = huffman.encode_string(input).unwrap();
//! let output = huffman.decode_string(bits).unwrap();
//! assert_eq!(input, &output);
//! ```

use alloc::{boxed::Box, string::String, vec::Vec};
use core::{
    fmt::{Display, Formatter},
    hash::Hash,
};

use hashbrown::HashMap;

use crate::{
    display_bytes_as_hex_array,
    types::integer::{Integer, IntegerSerError, SignedState},
    utilities::{bits::Bits, cursor::Cursor},
};

///A struct to hold the conversions between a `T` and the huffman bits which represent it.
#[derive(Debug)]
pub struct Huffman<T: Hash + Eq + Clone> {
    to_bits: HashMap<T, Bits>,
    root: Node<T>,
}
//I tested tree traversal both ways, and in the end it made encoding like 2000% slower (300 nano -> 7 milli), but encoding like 90% faster (1 milli -> 100 nano), so that's the cause for the split approach

///A binary tree structure for use in creating the huffman encoding
#[derive(Debug, Eq, PartialEq, Hash)]
enum Node<T> {
    Leaf(T),
    Branch {
        left: Box<Node<T>>,
        right: Box<Node<T>>,
    },
}

impl<T: PartialEq> Node<T> {
    #[allow(dead_code)]
    pub fn find(&self, target: &T) -> Option<Bits> {
        self.find_internal(target, Bits::default())
    }

    #[allow(dead_code)]
    fn find_internal(&self, target: &T, so_far: Bits) -> Option<Bits> {
        match self {
            Self::Leaf(leaf) => {
                if leaf == target {
                    Some(so_far)
                } else {
                    None
                }
            }
            Self::Branch { left, right } => {
                let left_path = so_far.push_into_new(false);
                if let Some(path) = left.find_internal(target, left_path) {
                    return Some(path);
                }

                let right_path = so_far.push_into_new(true);
                if let Some(right_path) = right.find_internal(target, right_path) {
                    return Some(right_path);
                }

                None
            }
        }
    }

    fn leaf_contents(&self) -> Option<&T> {
        if let Node::Leaf(t) = self {
            Some(t)
        } else {
            None
        }
    }
}
impl Node<char> {
    fn ser(&self) -> Vec<u8> {
        match self {
            Self::Leaf(ch) => Integer::u32(*ch as u32 + 1).ser().1, //to ensure that if we see a null byte, we don't serialise it as our control character. NB: u32::MAX is not a valid character so we also don't actually lose anything - only readability in the binary format, but that's long gone
            Self::Branch { left, right } => {
                let mut res = vec![0]; //null byte means new branch
                res.extend(left.ser());
                res.extend(right.ser());
                res
            }
        }
    }

    fn deser(cursor: &mut Cursor<u8>) -> Result<Node<char>, HuffmanSerError> {
        let Some(ch) = cursor.next() else {
            return Err(HuffmanSerError::NotEnoughBytes);
        };

        if *ch == 0 {
            let left = Box::new(Self::deser(cursor)?);
            let right = Box::new(Self::deser(cursor)?);
            Ok(Node::Branch { left, right })
        } else {
            cursor.move_backwards(1);
            let ch: u32 = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
            let Ok(ch) = char::try_from(ch - 1) else {
                return Err(HuffmanSerError::InvalidCharacter(ch - 1));
            };

            Ok(Node::Leaf(ch))
        }
    }
}

///Any possible error which could occur with huffman coding.
#[derive(Debug)]
pub enum HuffmanSerError {
    ///An invalid character was deserialised - this should be very rare and can only occur when using the [`Huffman::deser`]
    InvalidCharacter(u32),
    ///An integer serialisation error - this can only occur during [`Huffman::deser`].
    Integer(IntegerSerError),
    ///Not enough bytes were present
    NotEnoughBytes,
    ///The node couldn't be deserialised as an expected character was missing
    InvalidNodeFormat { ex: char, found: u8 },
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
            HuffmanSerError::NotEnoughBytes => write!(f, "Not enough bytes"),
            HuffmanSerError::InvalidNodeFormat { ex, found } => write!(
                f,
                "Tried to deserialise node, expected: {} ({ex}) but found {found}",
                *ex as u32
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for HuffmanSerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Integer(i) => Some(i),
            _ => None,
        }
    }
}

///An internal min-heap structure which implements [`Iterator`] and is designed to always return the next smallest element based on the weight provided.
struct MinHeap<T> {
    ///NB: this list does **not** need to be ordered
    list: Vec<(T, usize)>,
}

impl<T> MinHeap<T> {
    ///Create a new list - the elements provided do not need to be ordered.
    ///
    /// The `T` represents the actual element you want to get out, and the `usize` is the weight of that item.
    pub fn new(list: Vec<(T, usize)>) -> Self {
        Self { list }
    }

    ///Add a new item to the list
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

impl<T: Eq + Hash + Clone> Huffman<T> {
    ///Convert an iterator into a node tree where less common elements are more left.
    ///
    /// Can return `None` if there aren't any elements.
    fn data_to_node_tree(data: impl Iterator<Item = T>) -> Option<Node<T>> {
        let mut frequency_table = HashMap::new();
        for ch in data {
            *frequency_table.entry(ch).or_default() += 1_usize;
        }
        Self::data_with_frequencies_to_node_tree(frequency_table)
    }

    ///Convert an iterator with weights already calculated to a node tree.
    ///
    /// NB: There is no unique-ness requirement in the list - the weights will get added together for the calculations.
    ///
    /// Can return `None` if no elements are provided.
    fn data_with_frequencies_to_node_tree(data: HashMap<T, usize>) -> Option<Node<T>> {
        if data.is_empty() {
            return None;
        }
        let mut frequency_table: HashMap<Node<T>, usize> = HashMap::new();
        for (ch, freq) in data {
            *frequency_table.entry(Node::Leaf(ch)).or_default() += freq;
        }
        //redo HM to ensure that uniqueness is preserved etc

        let mut min_heap: MinHeap<Node<T>> = MinHeap::new(frequency_table.into_iter().collect());

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

    ///Breadth-first traversal of the node, adding leaf nodes and their paths to the provided [`HashMap`]s.
    ///
    /// For an external call, the `bits_so_far` should be [`Bits::default`].
    fn add_node_to_table<U: Hash + Eq + Clone>(
        node: &Node<U>,
        to_bits: &mut HashMap<U, Bits>,
        bits_so_far: Bits,
    ) {
        match node {
            Node::Leaf(ch) => {
                to_bits.insert(ch.clone(), bits_so_far);
            }
            Node::Branch { left, right } => {
                let mut left_bits = bits_so_far.clone();
                let mut right_bits = bits_so_far;
                left_bits.push(true);
                right_bits.push(false);

                Self::add_node_to_table(left, to_bits, left_bits);
                Self::add_node_to_table(right, to_bits, right_bits);
            }
        }
    }

    ///Create a new Huffman tree with given data. Can return `None` if the provided iterator is empty.
    ///
    /// All the possible data that could be used should be provided. For example, if you are encoding scores out of 10, all 10 different scores should be in this iterator. It is also helpful to provide elements in frequencies similar to the expected frequencies in the data for the most efficient encoding to occur.
    ///
    ///Can return `None` if the iterator provided is empty.
    #[must_use]
    pub fn new(data: impl Iterator<Item = T>) -> Option<Self> {
        let mut to_bits = HashMap::new();

        let root = Self::data_to_node_tree(data)?;

        Self::add_node_to_table(&root, &mut to_bits, Bits::default());

        Some(Self { to_bits, root })
    }

    ///Encode a series of `T`s into a [`Bits`]. Will return `None` if any elements found in the iterator were not included in the original [`Huffman::new`] incantation.
    pub fn encode(&self, from: impl Iterator<Item = T>) -> Option<Bits> {
        let mut bits = Bits::default();

        for ch in from {
            let bits_to_add = self.to_bits.get(&ch).cloned()?;
            bits.push_many(bits_to_add);
        }

        Some(bits)
    }

    ///Decode a series of `T`s from a [`Bits`]. Will return `None` if a sequence in the `bits` cannot be found in the conversion tables calculated during the original [`Huffman::new`] incantation.
    #[must_use]
    pub fn decode(&self, bits: Bits) -> Option<Vec<T>> {
        let mut result = Vec::new();
        let mut current_node = &self.root;

        for next_direction in bits {
            let new_node;
            match current_node {
                Node::Leaf(_) => panic!("not sure what happens here lol"),
                Node::Branch { left, right } => {
                    let found = if next_direction { left } else { right };
                    if let Some(t) = found.leaf_contents().cloned() {
                        new_node = &self.root;
                        result.push(t);
                    } else {
                        new_node = found;
                    }
                }
            }

            current_node = new_node;
        }

        Some(result)
    }
}

impl Huffman<char> {
    ///Create a new huffman code based off a string.
    pub fn new_str(str: impl AsRef<str>) -> Option<Self> {
        Self::new(str.as_ref().chars())
    }

    ///Create a new huffman code based off the reuters corpus of english letter frequencies.
    #[allow(
        clippy::too_many_lines,
        clippy::unreadable_literal,
        clippy::missing_panics_doc
    )]
    #[must_use]
    pub fn new_with_english_frequencies() -> Self {
        //sourced from here: https://github.com/piersy/ascii-char-frequency-english
        //source code modified to not normalise the values tho
        let freqs_map = [
            (32_u8, 2643715),
            (101, 1358462),
            (116, 998648),
            (97, 966445),
            (110, 868336),
            (105, 864695),
            (111, 854979),
            (115, 818629),
            (114, 812926),
            (108, 507744),
            (100, 503130),
            (104, 413245),
            (99, 394475),
            (10, 308889),
            (117, 303678),
            (109, 286203),
            (112, 273927),
            (102, 248498),
            (103, 202023),
            (46, 174421),
            (121, 171873),
            (98, 163239),
            (119, 150923),
            (44, 136229),
            (118, 123365),
            (48, 93385),
            (107, 78030),
            (49, 77905),
            (83, 48747),
            (84, 48438),
            (67, 47133),
            (50, 43486),
            (56, 40276),
            (53, 39868),
            (65, 39088),
            (57, 38532),
            (120, 36389),
            (51, 34498),
            (73, 32991),
            (45, 32765),
            (54, 30291),
            (52, 29007),
            (55, 28783),
            (77, 28612),
            (66, 27432),
            (34, 24856),
            (39, 23790),
            (80, 21916),
            (69, 20413),
            (78, 20130),
            (70, 19253),
            (82, 17414),
            (68, 17241),
            (85, 16450),
            (113, 15912),
            (76, 15848),
            (71, 14689),
            (74, 13907),
            (72, 13809),
            (79, 12954),
            (87, 12698),
            (106, 9744),
            (122, 9092),
            (47, 8198),
            (60, 6959),
            (62, 6949),
            (75, 6008),
            (41, 5229),
            (40, 5219),
            (86, 4033),
            (89, 3975),
            (58, 1899),
            (81, 1578),
            (90, 1360),
            (88, 1037),
            (59, 117),
            (63, 73),
            (127, 49),
            (94, 35),
            (38, 32),
            (43, 24),
            (91, 11),
            (93, 10),
            (33, 8),
            (36, 8),
            (42, 7),
            (61, 4),
            (126, 3),
            (9, 2),
            (95, 2),
            (27, 1),
            (123, 1),
            (5, 1),
            (30, 1),
            (64, 1),
        ]
        .into_iter()
        .map(|(ch, f)| (char::from(ch), f))
        .collect();

        let root = Self::data_with_frequencies_to_node_tree(freqs_map).unwrap();

        let mut chars_to_bits = HashMap::new();

        Self::add_node_to_table(&root, &mut chars_to_bits, Bits::default());

        Self {
            root,
            to_bits: chars_to_bits,
        }
    }

    ///Encode a string into a [`Bits`]. Will return `None` if it encounters a new character.
    pub fn encode_string(&self, str: impl AsRef<str>) -> Option<Bits> {
        self.encode(str.as_ref().chars())
    }

    ///Decode a string from a [`Bits`]. Will return `None` if it cannot parse the [`Bits`].
    #[must_use]
    pub fn decode_string(&self, bits: Bits) -> Option<String> {
        Some(self.decode(bits)?.into_iter().collect())
    }

    ///Serialise the huffman tables into a series of bytes using [`Integer::ser`].
    ///
    /// Encoding Scheme:
    ///
    /// Serialises using a recursive algorithm that traverses the tree to produce something like this: ((a,(b,c)),(d,e)) - the brackets symbolise a node parent, and the left node comes left
    #[must_use]
    pub fn ser(&self) -> Vec<u8> {
        self.root.ser()
    }

    ///Deserialise a [`Cursor`] into a [`Huffman`] using [`Node::deser`]
    ///
    /// ## Errors
    /// See [`Node::deser`].
    pub fn deser(bytes: &mut Cursor<u8>) -> Result<Self, HuffmanSerError> {
        let root = Node::deser(bytes)?;

        let mut to_bits = HashMap::new();
        Self::add_node_to_table(&root, &mut to_bits, Bits::default());

        Ok(Self { to_bits, root })
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use proptest::{prop_assert_eq, proptest};

    use crate::utilities::{
        bits::Bits,
        cursor::Cursor,
        huffman::{Huffman, Node},
    };

    #[test]
    fn nodes_from_empty_string() {
        let huffman = Huffman::data_to_node_tree("".chars());
        assert!(huffman.is_none());
    }

    #[test]
    fn nodes_from_one_char_repeated() {
        let huffman =
            Huffman::data_to_node_tree("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".chars()).unwrap();
        let Node::Leaf(ch) = huffman else {
            panic!("didn't find leaf node at root");
        };
        assert_eq!(ch, 'a');
    }

    #[test]
    fn test_encode_decode_five_characters() {
        let data = "abcdeabcdabcabaaaaaa";
        let huffman = Huffman::new_str(data).unwrap();

        let encoded = huffman.encode_string(data).unwrap();
        let decoded = huffman.decode_string(encoded).unwrap();

        assert_eq!(data, decoded);
    }

    proptest! {
        #[test]
        fn doesnt_crash (s in "\\PC*") {
            let _ = Huffman::new_str(s);
        }

        #[test]
        fn works_on_arbritrary_ascii_strings (s in "[a-z]+[A-Z0-9]+") {
            let huffman = Huffman::new_str(&s).expect("unable to get huffman");

            let encoded = huffman.encode_string(&s).expect("unable to encode");
            let decoded = huffman.decode_string(encoded).expect("unable to decode");

            prop_assert_eq!(s, decoded);
        }

        #[test]
        fn works_on_arbritrary_strings (s in "..+") {
            let huffman = Huffman::new_str(&s).expect("unable to get huffman");

            let encoded = huffman.encode_string(&s).expect("unable to encode");
            let decoded = huffman.decode_string(encoded).expect("unable to decode");

            prop_assert_eq!(s, decoded);
        }


        #[test]
        fn ser_works_on_arbritrary_strings (s in "..+") {
            let huffman = Huffman::new_str(&s).expect("unable to get huffman");
            let encoded = huffman.encode_string(&s).expect("unable to encode");

            let serialised_bits = encoded.ser();
            drop(encoded);
            let serialised_huffman = huffman.ser();
            drop(huffman);

            let deserialised_huffman = Huffman::deser(&mut Cursor::new(&serialised_huffman)).expect("unable to deser huffman");
            let deserialised_bits = Bits::deser(&mut Cursor::new(&serialised_bits)).expect("unable to deser huffman");

            let decoded = deserialised_huffman.decode_string(deserialised_bits).expect("unable to decode");
            prop_assert_eq!(s, decoded);
        }
    }
}
