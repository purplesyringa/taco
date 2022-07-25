use crate::autocompress::AutoCompressOpts;
use crate::bits::Bits;
use crate::varint::compress_varint;

use std::hash::Hash;

#[derive(Debug)]
pub enum Engine {
    VarInt,
    FixedInt {
        bias: i128,
        length: usize,
    },
    SpecificHuffman {
        alphabet_engine: Box<Engine>,
        alphabet_data: Bits,
        tree: Bits,
    },
    CanonicalHuffman {
        alphabet_engine: Box<Engine>,
        alphabet_data: Bits,
        lengths_engine: Box<Engine>,
        lengths_data: Bits,
    },
    String {
        chars: Box<Engine>,
    },
    StringConcat {
        words: Box<Engine>,
        separator: char,
    },
    IntSet {
        min: Box<Engine>,
        max: Box<Engine>,
        unique: bool,
    },
    Vec {
        length: Box<Engine>,
        item: Box<Engine>,
    },
    VecRLE {
        length: Box<Engine>,
        item: Box<Engine>,
    },
    CategorySplit {
        categories: Vec<Engine>,
        category: Box<Engine>,
    },
    Constant {
        engine: Box<Engine>,
        data: Bits,
    },
    Alphabet {
        alphabet_engine: Box<Engine>,
        alphabet_data: Bits,
        index: Box<Engine>,
    },
    StringifiedInt {
        inner: Box<Engine>,
    },
    StringifiedDecimal {
        inner: Box<Engine>,
        precision: Box<Engine>,
    },
}

impl Engine {
    pub fn weight(&self) -> usize {
        self.to_bits().len()
    }

    pub fn to_bits(&self) -> Bits {
        let mut bits = Bits::new();
        self.push_to_bits(&mut bits);
        bits
    }

    pub fn push_to_bits(&self, bits: &mut Bits) {
        match self {
            Self::VarInt => {
                bits.push(false);
                bits.push(false);
                bits.push(false);
                bits.push(false);
            }
            Self::FixedInt { bias, length } => {
                bits.push(false);
                bits.push(false);
                bits.push(false);
                bits.push(true);
                bits.extend(&compress_varint(*bias));
                bits.extend(&compress_varint(*length as i128));
            }
            Self::SpecificHuffman {
                alphabet_engine,
                alphabet_data,
                tree,
            } => {
                bits.push(false);
                bits.push(false);
                bits.push(true);
                bits.push(false);
                alphabet_engine.push_to_bits(bits);
                bits.extend(&alphabet_data);
                bits.extend(&tree);
            }
            Self::CanonicalHuffman {
                alphabet_engine,
                alphabet_data,
                lengths_engine,
                lengths_data,
            } => {
                bits.push(false);
                bits.push(false);
                bits.push(true);
                bits.push(true);
                alphabet_engine.push_to_bits(bits);
                bits.extend(&alphabet_data);
                lengths_engine.push_to_bits(bits);
                bits.extend(&lengths_data);
            }
            Self::String { chars } => {
                bits.push(false);
                bits.push(true);
                bits.push(false);
                bits.push(false);
                chars.push_to_bits(bits);
            }
            Self::StringConcat { words, separator } => {
                bits.push(false);
                bits.push(true);
                bits.push(false);
                bits.push(true);
                bits.extend(&compress_varint(*separator as i128));
                words.push_to_bits(bits);
            }
            Self::IntSet { min, max, unique } => {
                bits.push(false);
                bits.push(true);
                bits.push(true);
                bits.push(*unique);
                min.push_to_bits(bits);
                max.push_to_bits(bits);
            }
            Self::Vec { length, item } => {
                bits.push(true);
                bits.push(false);
                bits.push(false);
                bits.push(false);
                length.push_to_bits(bits);
                item.push_to_bits(bits);
            }
            Self::VecRLE { length, item } => {
                bits.push(true);
                bits.push(false);
                bits.push(false);
                bits.push(true);
                length.push_to_bits(bits);
                item.push_to_bits(bits);
            }
            Self::CategorySplit {
                categories,
                category,
            } => {
                bits.push(true);
                bits.push(false);
                bits.push(true);
                bits.extend(&compress_varint(categories.len() as i128));
                for cat in categories {
                    cat.push_to_bits(bits);
                }
                category.push_to_bits(bits);
            }
            Self::Constant { engine, data } => {
                bits.push(true);
                bits.push(true);
                bits.push(false);
                bits.push(false);
                engine.push_to_bits(bits);
                bits.extend(data);
            }
            Self::Alphabet {
                alphabet_engine,
                alphabet_data,
                index,
            } => {
                bits.push(true);
                bits.push(true);
                bits.push(false);
                bits.push(true);
                alphabet_engine.push_to_bits(bits);
                bits.extend(alphabet_data);
                index.push_to_bits(bits);
            }
            Self::StringifiedInt { inner } => {
                bits.push(true);
                bits.push(true);
                bits.push(true);
                bits.push(false);
                inner.push_to_bits(bits);
            }
            Self::StringifiedDecimal { inner, precision } => {
                bits.push(true);
                bits.push(true);
                bits.push(true);
                bits.push(true);
                inner.push_to_bits(bits);
                precision.push_to_bits(bits);
            }
        }
    }
}

pub struct CompressedData {
    pub engine: Engine,
    pub binary_data: Vec<Bits>,
}

impl CompressedData {
    pub fn weight(&self) -> usize {
        self.binary_data
            .iter()
            .map(|bits| bits.len())
            .sum::<usize>()
            + self.engine.weight()
    }
}

pub trait Compress: Eq + Hash + std::fmt::Debug {
    fn compress_multiple(objs: &[&Self], opts: AutoCompressOpts) -> CompressedData;
    fn split_categories(objs: &[&Self]) -> Option<Vec<Vec<usize>>>;
}

impl<T: Compress> Compress for &T {
    fn compress_multiple(objs: &[&Self], opts: AutoCompressOpts) -> CompressedData {
        let refs: Vec<&T> = objs.iter().map(|obj| **obj).collect();
        T::compress_multiple(&refs, opts)
    }
    fn split_categories(objs: &[&Self]) -> Option<Vec<Vec<usize>>> {
        let refs: Vec<&T> = objs.iter().map(|obj| **obj).collect();
        T::split_categories(&refs)
    }
}
