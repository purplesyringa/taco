mod bits;
mod varint;
mod compress;
mod compress_int;
mod compress_str;
mod compress_vec;
mod split;

use compress::{CompressedData, AutoCompressOpts, autocompress_one};

fn main() {
    let compressed = autocompress_one(&"6
4 6 3
12 9 8
3 3 2
8 8
3 3 2
9 5
4 5 2
10 11
5 4 2
9 11
10 10 3
11 45 14".to_string(), AutoCompressOpts::default());

    let mut result = compressed.engine.to_bits();
    result.extend(&compressed.binary_data[0]);

    println!("{:?}", compressed.engine);
    println!("{:?}", result.to_bytes());
}
