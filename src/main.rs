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

use autocompress::{autocompress_one, AutoCompressOpts};

fn main() {
    let mut args = std::env::args();
    args.next();
    let path = args.next().expect("Need file path as first argument");

    let s = std::fs::read_to_string(path).expect("Failed to read file");

    let compressed = autocompress_one(&s, AutoCompressOpts::default());

    let mut result = compressed.engine.to_bits();
    result.extend(&compressed.binary_data[0]);

    println!("{:?}", compressed.engine);
    println!("{:?}", result.to_bytes().len());
    // std::io::stdout()
    //     .write(&result.to_bytes())
    //     .expect("Failed to write to stdout");
}
