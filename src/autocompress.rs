use crate::bits::Bits;
use crate::compress::{Compress, CompressedData, Engine, MultiCompressedData};
use crate::huffman::huffman;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct AutoCompressOpts {
    pub enable_dedup_and_categories: bool,
    pub enable_stateful: bool,
}

impl Default for AutoCompressOpts {
    fn default() -> Self {
        AutoCompressOpts {
            enable_dedup_and_categories: true,
            enable_stateful: true,
        }
    }
}

fn try_autocompress_dedup<T: Compress>(objs: &[&T]) -> Option<MultiCompressedData> {
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
        let data = autocompress_one(objs[0], AutoCompressOpts::default());
        Some(MultiCompressedData {
            engine: Engine::Constant {
                engine: Box::new(data.engine),
                data: data.binary_data,
            },
            binary_data: vec![Bits::new(); objs.len()],
        })
    } else if values_list.len() < objs.len().min(objs.len() / 2 + 3) {
        // Reasonable to compress

        // Huffman encoding
        let huffman_encoded = huffman(objs, AutoCompressOpts::default());

        // Alphabet-based encoding
        let mut values_compressed = autocompress(&values_list, AutoCompressOpts::default());
        let indices_refs: Vec<&usize> = indices.iter().collect();
        let indices_compressed = autocompress(
            &indices_refs,
            AutoCompressOpts {
                enable_dedup_and_categories: false,
                enable_stateful: true,
            },
        );
        let alphabet_encoded = MultiCompressedData {
            engine: Engine::Alphabet {
                alphabet_engine: Box::new(values_compressed.engine),
                alphabet_data: values_compressed.binary_data.pop().unwrap(),
                index: Box::new(indices_compressed.engine),
            },
            binary_data: indices_compressed.binary_data,
        };

        if huffman_encoded.weight() < alphabet_encoded.weight() {
            Some(huffman_encoded)
        } else {
            Some(alphabet_encoded)
        }
    } else {
        None
    }
}

fn try_autocompress_categories<T: Compress>(objs: &[&T]) -> Option<MultiCompressedData> {
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
    let category_by_obj_compressed = autocompress(
        &category_by_obj_refs,
        AutoCompressOpts {
            enable_dedup_and_categories: false,
            enable_stateful: true,
        },
    );

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

    Some(MultiCompressedData {
        engine: Engine::CategorySplit {
            categories: categories_engines,
            category: Box::new(category_by_obj_compressed.engine),
        },
        binary_data,
    })
}

fn autocompress_stateful<T: Compress>(objs: &[&T], opts: AutoCompressOpts) -> MultiCompressedData {
    let data = objs.to_vec().compress(opts);
    MultiCompressedData {
        engine: Engine::Stateful {
            inner: Box::new(data.engine),
            data: data.binary_data,
        },
        binary_data: vec![Bits::new(); objs.len()],
    }
}

static mut cc: usize = 0usize;

pub fn autocompress<T: Compress>(objs: &[&T], opts: AutoCompressOpts) -> MultiCompressedData {
    if objs.is_empty() {
        return MultiCompressedData {
            engine: Engine::VarInt,
            binary_data: Vec::new(),
        };
    }

    // println!(
    //     "{}autocompress {objs:?} {opts:?}",
    //     " ".repeat(unsafe { cc })
    // );
    unsafe {
        cc += 1;
    }

    if opts.enable_dedup_and_categories {
        if let Some(data) = try_autocompress_dedup(objs) {
            unsafe {
                cc -= 1;
            }
            return data;
        }

        if let Some(data) = try_autocompress_categories(objs) {
            unsafe {
                cc -= 1;
            }
            // This may be less efficient than direct compression
            let data_direct = T::compress_multiple(objs, opts);
            if data_direct.weight() < data.weight() {
                return data_direct;
            }
            return data;
        }
    }

    if opts.enable_stateful && objs.len() > 1 {
        let data = autocompress_stateful(objs, opts);
        unsafe {
            cc -= 1;
        }
        // This may be less efficient than direct compression
        let data_direct = T::compress_multiple(objs, opts);
        if data_direct.weight() < data.weight() {
            return data_direct;
        }
        return data;
    }

    let data = T::compress_multiple(objs, opts);
    unsafe {
        cc -= 1;
    }
    data
}

pub fn autocompress_one<T: Compress>(obj: &T, opts: AutoCompressOpts) -> CompressedData {
    // println!(
    //     "{}autocompress_one {obj:?} {opts:?}",
    //     " ".repeat(unsafe { cc })
    // );
    unsafe {
        cc += 1;
    }
    let data = obj.compress(opts);
    unsafe {
        cc -= 1;
    }
    data
}
