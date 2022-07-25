#![feature(specialization)]

use std::io::Write;

mod autocompress;
mod bits;
mod compress;
mod compress_int;
mod compress_str;
mod compress_vec;
mod huffman;
mod split;
mod varint;

use autocompress::{autocompress, autocompress_one, AutoCompressOpts};

fn main() {
    let mut args = std::env::args();
    args.next();

    let mut strings = Vec::new();
    while let Some(path) = args.next() {
        let s = std::fs::read_to_string(path).expect("Failed to read file");
        strings.push(s);
    }

    let strings_refs: Vec<&String> = strings.iter().collect();

    let compressed = autocompress(&strings_refs, AutoCompressOpts::default());

    // let result = compressed.engine.to_bits();
    // result.extend(&compressed.binary_data);

    // println!("{:?}", compressed.engine);
    // println!("{:?}", result.to_bytes().len());
    println!("{:?}", compressed.weight());
    // std::io::stdout()
    //     .write(&result.to_bytes())
    //     .expect("Failed to write to stdout");
}
