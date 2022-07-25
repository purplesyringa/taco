use crate::autocompress::{autocompress_one, AutoCompressOpts};
use crate::bits::Bits;
use crate::compress::{Compress, Engine, MultiCompressedData};
use crate::varint::{compress_fixint, get_bit_length};
use std::collections::{BinaryHeap, HashMap, HashSet};

trait Huffman {
    fn huffman(objs: &[&Self], opts: AutoCompressOpts) -> MultiCompressedData;
}

impl<T: Compress> Huffman for T {
    default fn huffman(objs: &[&T], opts: AutoCompressOpts) -> MultiCompressedData {
        huffman_unordered(objs, opts)
    }
}

impl<T: Compress + Ord> Huffman for T {
    fn huffman(objs: &[&T], opts: AutoCompressOpts) -> MultiCompressedData {
        let unordered = huffman_unordered(objs, opts);
        let ordered = huffman_ordered(objs, opts);
        if unordered.weight() < ordered.weight() {
            unordered
        } else {
            ordered
        }
    }
}

pub fn huffman<T: Compress>(objs: &[&T], opts: AutoCompressOpts) -> MultiCompressedData {
    <T as Huffman>::huffman(objs, opts)
}

enum Tree {
    Leaf(usize),
    Branch(Box<Tree>, Box<Tree>),
}

struct HeapItem {
    weight: usize,
    tree: Box<Tree>,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.weight == other.weight
    }
}
impl Eq for HeapItem {}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.weight.partial_cmp(&self.weight)
    }
}
impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.weight.cmp(&self.weight)
    }
}

pub fn huffman_ordered<T: Compress + Ord>(
    objs: &[&T],
    opts: AutoCompressOpts,
) -> MultiCompressedData {
    // The alphabet is sorted, which makes it easier to encode, but the tree has to be stored
    let mut alphabet: Vec<&T> = objs.to_vec();
    alphabet.sort();
    alphabet.dedup();

    let (alphabet_representations, tree) = build_tree(&objs, &alphabet);

    let mut tree_enc = Bits::new();
    let bit_length = get_bit_length(alphabet.len() as u128);
    fn walk(tree: &Tree, tree_enc: &mut Bits, bit_length: usize) {
        match tree {
            Tree::Leaf(i) => {
                tree_enc.push(true);
                tree_enc.extend(&compress_fixint(*i as u128, bit_length));
            }
            Tree::Branch(a, b) => {
                tree_enc.push(false);
                walk(&a, tree_enc, bit_length);
                walk(&b, tree_enc, bit_length);
            }
        }
    }
    walk(&tree, &mut tree_enc, bit_length);

    let mut alphabet_offset: HashMap<&T, usize> = HashMap::new();
    for (i, obj) in alphabet.iter().enumerate() {
        alphabet_offset.insert(obj, i);
    }

    let alphabet_compressed = autocompress_one(&alphabet, opts);

    MultiCompressedData {
        engine: Engine::SpecificHuffman {
            alphabet_engine: Box::new(alphabet_compressed.engine),
            alphabet_data: alphabet_compressed.binary_data,
            tree: tree_enc,
        },
        binary_data: objs
            .iter()
            .map(|obj| alphabet_representations[alphabet_offset[obj]].clone())
            .collect(),
    }
}

pub fn huffman_unordered<T: Compress>(objs: &[&T], opts: AutoCompressOpts) -> MultiCompressedData {
    // The alphabet is reordered, but the tree is implicit (canonical)
    let alphabet: HashSet<&T> = objs.iter().cloned().collect();
    let alphabet: Vec<&T> = alphabet.into_iter().collect();

    // Generate code lengths. What alphabet order we use doesn't matter, only counts do
    let (alphabet_representations, _) = build_tree(objs, &alphabet);

    let mut code_lengths: Vec<(usize, usize)> = alphabet_representations
        .into_iter()
        .enumerate()
        .map(|(i, bits)| (bits.len(), i))
        .collect();
    code_lengths.sort();

    let lengths: Vec<&usize> = code_lengths.iter().map(|(len, _)| len).collect();
    let lengths_compressed = autocompress_one(&lengths, opts);

    let alphabet: Vec<&T> = code_lengths.iter().map(|(_, i)| alphabet[*i]).collect();

    let alphabet_compressed = autocompress_one(&alphabet, opts);

    let mut alphabet_representations: HashMap<&T, Bits> = HashMap::new();
    let mut code = Bits::new();
    for (i, obj) in alphabet.into_iter().enumerate() {
        while code.len() < *lengths[i] {
            code.push(false);
        }
        if i > 0 {
            // Increment
            let mut cnt = 0usize;
            while code.pop().unwrap() {
                cnt += 1;
            }
            code.push(true);
            for _ in 0..cnt {
                code.push(false);
            }
        }
        alphabet_representations.insert(obj, code.clone());
    }

    MultiCompressedData {
        engine: Engine::CanonicalHuffman {
            alphabet_engine: Box::new(alphabet_compressed.engine),
            alphabet_data: alphabet_compressed.binary_data,
            lengths_engine: Box::new(lengths_compressed.engine),
            lengths_data: lengths_compressed.binary_data,
        },
        binary_data: objs
            .iter()
            .map(|obj| alphabet_representations[obj].clone())
            .collect(),
    }
}

fn build_tree<'a, T: Compress>(objs: &[&'a T], alphabet: &[&'a T]) -> (Vec<Bits>, Box<Tree>) {
    let mut counts: HashMap<&T, usize> = HashMap::new();
    for obj in objs {
        *counts.entry(obj).or_default() += 1;
    }

    let alphabet_counts: Vec<usize> = alphabet.iter().map(|obj| counts[obj]).collect();
    let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();
    for (i, count) in alphabet_counts.iter().enumerate() {
        heap.push(HeapItem {
            weight: *count,
            tree: Box::new(Tree::Leaf(i)),
        });
    }

    while heap.len() > 1 {
        let a = heap.pop().unwrap();
        let b = heap.pop().unwrap();
        heap.push(HeapItem {
            weight: a.weight + b.weight,
            tree: Box::new(Tree::Branch(a.tree, b.tree)),
        });
    }

    let mut alphabet_representations = vec![Bits::new(); alphabet.len()];
    let mut prefix = Bits::new();

    fn walk(tree: &Tree, alphabet_representations: &mut Vec<Bits>, prefix: &mut Bits) {
        match tree {
            Tree::Leaf(i) => {
                alphabet_representations[*i] = prefix.clone();
            }
            Tree::Branch(a, b) => {
                prefix.push(false);
                walk(&a, alphabet_representations, prefix);
                prefix.pop();
                prefix.push(true);
                walk(&b, alphabet_representations, prefix);
                prefix.pop();
            }
        }
    }

    let tree = heap.pop().unwrap().tree;
    walk(&tree, &mut alphabet_representations, &mut prefix);

    (alphabet_representations, tree)
}
