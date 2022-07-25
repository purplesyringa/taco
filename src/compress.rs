use std::collections::HashMap;
use std::hash::Hash;
use crate::bits::Bits;
use crate::varint::compress_varint;

#[derive(Debug)]
pub enum Engine {
    VarInt,
    BiasedVarInt { bias: i128 },
    String { chars: Box<Engine> },
    StringConcat { words: Box<Engine>, separator: char },
    Vec { length: Box<Engine>, item: Box<Engine> },
    CategorySplit { categories: Vec<Engine>, category: Box<Engine> },
    Constant { engine: Box<Engine>, data: Bits },
    Alphabet { alphabet_engine: Box<Engine>, alphabet_data: Bits, index: Box<Engine> }
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
            }
            Self::BiasedVarInt { bias } => {
                bits.push(false);
                bits.push(false);
                bits.push(true);
                bits.extend(&compress_varint(*bias));
            }
            Self::String { chars } => {
                bits.push(false);
                bits.push(true);
                bits.push(false);
                chars.push_to_bits(bits);
            }
            Self::StringConcat { words, separator } => {
                bits.push(false);
                bits.push(true);
                bits.push(true);
                bits.extend(&compress_varint(*separator as i128));
                words.push_to_bits(bits);
            }
            Self::Vec { length, item } => {
                bits.push(true);
                bits.push(false);
                bits.push(false);
                length.push_to_bits(bits);
                item.push_to_bits(bits);
            }
            Self::CategorySplit { categories, category } => {
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
                engine.push_to_bits(bits);
                bits.extend(data);
            }
            Self::Alphabet { alphabet_engine, alphabet_data, index } => {
                bits.push(true);
                bits.push(true);
                bits.push(true);
                alphabet_engine.push_to_bits(bits);
                bits.extend(alphabet_data);
                index.push_to_bits(bits);
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
        self.binary_data.iter().map(|bits| bits.len()).sum::<usize>() + self.engine.weight()
    }
}

pub trait Compress: Eq + Hash + std::fmt::Debug {
    fn compress_multiple(objs: &[&Self], opts: AutoCompressOpts) -> CompressedData;
    fn split_categories(objs: &[&Self]) -> Option<Vec<Vec<usize>>>;
}

#[derive(Debug)]
pub struct AutoCompressOpts {
    enable_dedup_and_categories: bool,
}

impl Default for AutoCompressOpts {
    fn default() -> Self {
        AutoCompressOpts {
            enable_dedup_and_categories: true,
        }
    }
}

fn try_autocompress_dedup<T: Compress>(objs: &[&T]) -> Option<CompressedData> {
    let mut values_list = Vec::new();
    let mut index_of_value = HashMap::new();
    let mut indices = Vec::with_capacity(objs.len());
    for x in objs {
        if let Some(i) = index_of_value.get(x) {
            indices.push(*i);
        } else {
            indices.push(values_list.len());
            index_of_value.insert(*x, values_list.len());
            values_list.push(*x);
        }
    }

    if objs.len() > 1 && values_list.len() == 1 {
        // Constant
        let mut data = autocompress_one(objs[0], AutoCompressOpts::default());
        Some(CompressedData {
            engine: Engine::Constant {
                engine: Box::new(data.engine),
                data: data.binary_data.pop().unwrap(),
            },
            binary_data: vec![Bits::new(); objs.len()]
        })
    } else if values_list.len() < objs.len().min(objs.len() / 2 + 3) {
        // Reasonable to compress
        let mut values_compressed = autocompress(&values_list, AutoCompressOpts::default());
        let indices_refs: Vec<&usize> = indices.iter().collect();
        let indices_compressed = autocompress(&indices_refs, AutoCompressOpts {
            enable_dedup_and_categories: false,
        });
        Some(CompressedData {
            engine: Engine::Alphabet {
                alphabet_engine: Box::new(values_compressed.engine),
                alphabet_data: values_compressed.binary_data.pop().unwrap(),
                index: Box::new(indices_compressed.engine),
            },
            binary_data: indices_compressed.binary_data,
        })
    } else {
        None
    }
}

fn try_autocompress_categories<T: Compress>(objs: &[&T]) -> Option<CompressedData> {
    let categories = T::split_categories(objs)?;
    if categories.len() < 2 {
        return None;
    }

    let mut category_by_obj = vec![0; objs.len()];
    for (i, category) in categories.iter().enumerate() {
        for j in category {
            category_by_obj[*j] = i;
        }
    }
    let category_by_obj_refs: Vec<&usize> = category_by_obj.iter().collect();
    let category_by_obj_compressed = autocompress(&category_by_obj_refs, AutoCompressOpts {
        enable_dedup_and_categories: false,
    });

    let mut binary_data = category_by_obj_compressed.binary_data;

    let mut categories_engines = Vec::with_capacity(categories.len());
    for category in categories.into_iter() {
        let category_objs: Vec<&T> = category.iter().map(|j| objs[*j]).collect();
        let data_compressed = autocompress(&category_objs, AutoCompressOpts::default());
        categories_engines.push(data_compressed.engine);
        for (i, j) in category.iter().enumerate() {
            binary_data[*j].extend(&data_compressed.binary_data[i]);
        }
    }

    Some(CompressedData {
        engine: Engine::CategorySplit {
            categories: categories_engines,
            category: Box::new(category_by_obj_compressed.engine),
        },
        binary_data
    })
}

static mut cc: usize = 0usize;

pub fn autocompress<T: Compress>(objs: &[&T], opts: AutoCompressOpts) -> CompressedData {
    println!("{}autocompress {objs:?} {opts:?}", " ".repeat(unsafe{cc}));
    unsafe{cc += 1;}
    if opts.enable_dedup_and_categories {
        if let Some(data) = try_autocompress_dedup(objs) {
            unsafe{cc -= 1;}
            return data;
        }
        if let Some(data) = try_autocompress_categories(objs) {
            unsafe{cc -= 1;}
            // This may be less efficient than direct compression
            let data_direct = T::compress_multiple(objs, opts);
            if data_direct.weight() < data.weight() {
                return data_direct;
            }
            return data;
        }
    }
    let data = T::compress_multiple(objs, opts);
    if data.binary_data.len() != objs.len() {
        panic!("Length mismatch");
    }
    unsafe{cc -= 1;}
    data
}

pub fn autocompress_one<T: Compress>(obj: &T, opts: AutoCompressOpts) -> CompressedData {
    autocompress(&[obj], opts)
}
