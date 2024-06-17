use alloc::{boxed::Box, string::String, vec::Vec};

use hashbrown::HashMap;

use crate::utilities::bits::Bits;

#[derive(Debug, Clone)]
pub struct Huffman {
    chars_to_bits: HashMap<char, Bits>,
    bits_to_chars: HashMap<Bits, char>,
}

#[derive(Debug, Clone)]
enum Node {
    Leaf(char),
    Branch { left: Box<Node>, right: Box<Node> },
}

impl Huffman {
    fn to_node_tree(data: &str) -> Option<Node> {
        if data.len() < 2 {
            return None;
        }

        let mut frequency_table: HashMap<char, usize> = HashMap::new();
        for ch in data.chars() {
            *frequency_table.entry(ch).or_default() += 1_usize;
        }
        let mut frequency_vec: Vec<(char, usize)> = frequency_table.into_iter().collect();
        frequency_vec.sort_by_key(|(_, freq)| (usize::MAX - *freq)); //smallest items at the end

        let (least_frequent_ch, least_frequent_weight) = frequency_vec.pop().unwrap(); //checked for len earlier
        let Some((next_least_frequent_ch, next_least_frequent_weight)) = frequency_vec.pop() else {
            return Some(Node::Leaf(least_frequent_ch));
        };

        let mut node = Node::Branch {
            left: Box::new(Node::Leaf(next_least_frequent_ch)),
            right: Box::new(Node::Leaf(least_frequent_ch)),
        };
        let mut weight = least_frequent_weight + next_least_frequent_weight;

        while let Some((next_ch, next_weight)) = frequency_vec.pop() {
            if next_weight > weight || frequency_vec.is_empty() {
                let new_node = Node::Branch {
                    left: Box::new(Node::Leaf(next_ch)),
                    right: Box::new(node.clone()), //TODO: work out way to avoid this weird clone situation where it immediately gets replaced
                };

                weight += next_weight;
                node = new_node;
            } else {
                let (next_next_ch, next_next_weight) = frequency_vec.pop().unwrap(); //just checked for empty
                let new_node = if next_next_weight > next_weight {
                    Node::Branch {
                        left: Box::new(Node::Leaf(next_next_ch)),
                        right: Box::new(Node::Leaf(next_ch)),
                    }
                } else {
                    Node::Branch {
                        left: Box::new(Node::Leaf(next_ch)),
                        right: Box::new(Node::Leaf(next_next_ch)),
                    }
                };
                let next_sum_weight = next_weight + next_next_weight;
                let new_node = if next_sum_weight > weight {
                    Node::Branch {
                        left: Box::new(new_node),
                        right: Box::new(node.clone()),
                    }
                } else {
                    Node::Branch {
                        left: Box::new(node.clone()),
                        right: Box::new(new_node),
                    }
                };

                weight += next_sum_weight;
                node = new_node;
            }
        }
        Some(node)
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
        add_node_to_table(
            Self::to_node_tree(data.as_ref())?,
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
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use hashbrown::HashMap;
    use proptest::{prop_assert_eq, proptest};

    use crate::utilities::{
        bits::Bits,
        huffman::{Huffman, Node},
    };

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
    fn nodes_from_just_two_chars() {
        let huffman =
            Huffman::to_node_tree("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbaaaaaaaaabb")
                .unwrap();
        let Node::Branch { left, right } = huffman else {
            panic!("didn't find branch node at root");
        };

        let Node::Leaf(left) = *left else {
            panic!("didn't find leaf node at left");
        };
        let Node::Leaf(right) = *right else {
            panic!("didn't find leaf node at right");
        };

        assert_eq!(left, 'a');
        assert_eq!(right, 'b');
    }

    #[test]
    fn nodes_from_five_characters() {
        let huffman = Huffman::to_node_tree("abcdeabcdabcabaaaaaa").unwrap();
        let Node::Branch { left, right } = huffman else {
            panic!("didn't find branch node at root");
        };

        {
            let Node::Leaf(a) = *left else {
                panic!("didn't find leaf node at left");
            };
            assert_eq!(a, 'a');
        }
        {
            let Node::Branch { left, right } = *right else {
                panic!("didn't find branch node at right");
            };

            {
                let Node::Branch { left, right } = *left else {
                    panic!("didn't find branch node at right->left");
                };

                let Node::Leaf(b) = *left else {
                    panic!("didn't find leaf node at right->left->left");
                };
                let Node::Leaf(c) = *right else {
                    panic!("didn't find leaf node at right->left->right");
                };

                assert_eq!(b, 'b');
                assert_eq!(c, 'c');
            }

            {
                let Node::Branch { left, right } = *right else {
                    panic!("didn't find branch node at right->right");
                };

                let Node::Leaf(d) = *left else {
                    panic!("didn't find leaf node at right->right->left");
                };
                let Node::Leaf(e) = *right else {
                    panic!("didn't find leaf node at right->right->right");
                };

                assert_eq!(d, 'd');
                assert_eq!(e, 'e');
            }
        }
    }

    #[test]
    fn huffman_bits_from_five_characters() {
        let huffman = Huffman::new("abcdeabcdabcabaaaaaa").unwrap();

        let expected: HashMap<char, Bits> = [
            ('a', Bits::from([false])),
            ('b', Bits::from([true, false, false])),
            ('c', Bits::from([true, false, true])),
            ('d', Bits::from([true, true, false])),
            ('e', Bits::from([true, true, true])),
        ]
        .into_iter()
        .collect();

        assert_eq!(huffman.chars_to_bits, expected);
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
