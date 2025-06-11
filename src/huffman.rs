use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq)]
enum NodeType {
    Leaf(char),
    Internal(Rc<Node>, Rc<Node>),
}

#[derive(Debug)]
struct Node {
    node_type: NodeType,
    freq: u32,
}

impl Node {
    fn new_leaf(symbol: char, freq: u32) -> Self {
        Node {
            node_type: NodeType::Leaf(symbol),
            freq,
        }
    }

    fn new_internal(left: Rc<Node>, right: Rc<Node>) -> Self {
        Node {
            node_type: NodeType::Internal(left.clone(), right.clone()),
            freq: left.freq + right.freq,
        }
    }
}

impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.freq == other.freq
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.freq.cmp(&self.freq)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Huffman<'a> {
    root: Option<Rc<Node>>,
    codec_dict: HashMap<char, String>,
    input: &'a str,
}

impl<'a> Huffman<'a> {
    pub fn from(input: &'a str) -> Self {
        let mut freq_dict: BTreeMap<char, u32> = BTreeMap::new();
        for symbol in input.chars() {
            *freq_dict.entry(symbol).or_default() += 1;
        }

        let mut pqueue = BinaryHeap::new();
        for (&symbol, &freq) in freq_dict.iter() {
            pqueue.push(Rc::new(Node::new_leaf(symbol, freq)));
        }

        while pqueue.len() > 1 {
            let ln = pqueue.pop().expect("checked with while loop condition");
            let rn = pqueue.pop().expect("checked with while loop condition");

            let internal = Rc::new(Node::new_internal(ln, rn));
            pqueue.push(internal);
        }

        let root = pqueue.pop();
        let mut codec_dict = HashMap::new();
        match root {
            Some(root) => {
                fill(&root, String::new(), &mut codec_dict);
                Self {
                    input,
                    codec_dict,
                    root: Some(root),
                }
            }
            None => Self {
                input,
                root: None,
                codec_dict: codec_dict,
            },
        }
    }

    pub fn encode(&self) -> String {
        let mut encoded = String::new();
        for symbol in self.input.chars() {
            if let Some(code) = self.codec_dict.get(&symbol) {
                encoded += code;
            }
        }

        encoded
    }

    pub fn decode(&self, encoded: &str) -> String {
        let mut decoded = String::new();

        // Handle edge cases
        if encoded.is_empty() {
            return decoded;
        }

        let Some(root) = &self.root else {
            return decoded; // No tree means no decoding possible
        };

        // Special case: single character tree
        if let NodeType::Leaf(symbol) = &root.node_type {
            // For single character, each bit represents one occurrence
            for _ in encoded.chars() {
                decoded.push(*symbol);
            }
            return decoded;
        }

        let mut current_node = root;

        for bit_char in encoded.chars() {
            match bit_char {
                '0' => {
                    if let NodeType::Internal(left, _) = &current_node.node_type {
                        current_node = left;
                    } else {
                        // This shouldn't happen with valid encoding
                        break;
                    }
                }
                '1' => {
                    if let NodeType::Internal(_, right) = &current_node.node_type {
                        current_node = right;
                    } else {
                        // This shouldn't happen with valid encoding
                        break;
                    }
                }
                _ => unreachable!("encoded format must be binary"),
            }

            if let NodeType::Leaf(symbol) = &current_node.node_type {
                decoded.push(*symbol);
                current_node = root;
            }
        }

        decoded
    }
}

fn fill(node: &Node, code: String, dict: &mut HashMap<char, String>) {
    match &node.node_type {
        NodeType::Leaf(symbol) => {
            let _ = dict.insert(*symbol, code);
        }
        NodeType::Internal(ln, rn) => {
            fill(ln, format!("{}0", code), dict);
            fill(rn, format!("{}1", code), dict);
        }
    }
}

#[allow(dead_code)]
fn print_tree(node: &Node, code: String) {
    match &node.node_type {
        NodeType::Leaf(symbol) => {
            println!("Sybmol: '{}': {} (freq: {})", symbol, code, node.freq)
        }
        NodeType::Internal(ln, rn) => {
            print_tree(ln, format!("{}0", code));
            print_tree(rn, format!("{}1", code));
        }
    }
}
