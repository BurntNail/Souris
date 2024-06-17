use alloc::{boxed::Box, vec::Vec};

use hashbrown::HashMap;

use crate::utilities::bits::Bits;

#[derive(Debug, Clone)]
pub struct Huffman {
    conversion_table: HashMap<char, Bits>,
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
        fn add_node_to_table(node: Node, table: &mut HashMap<char, Bits>, bits_so_far: Bits) {
            match node {
                Node::Leaf(ch) => {
                    table.insert(ch, bits_so_far);
                }
                Node::Branch { left, right } => {
                    let mut left_bits = bits_so_far.clone();
                    let mut right_bits = bits_so_far.clone();
                    left_bits.push(false);
                    right_bits.push(true);

                    add_node_to_table(*left, table, left_bits);
                    add_node_to_table(*right, table, right_bits);
                }
            }
        }

        let mut conversion_table = HashMap::new();
        add_node_to_table(
            Self::to_node_tree(data.as_ref())?,
            &mut conversion_table,
            Bits::default(),
        );

        Some(Self { conversion_table })
    }
}

#[cfg(test)]
mod tests {
    use hashbrown::HashMap;

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

        assert_eq!(huffman.conversion_table, expected);
    }
}
