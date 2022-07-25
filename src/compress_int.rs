use crate::autocompress::AutoCompressOpts;
use crate::compress::{Compress, CompressedData, Engine};
use crate::varint::{compress_fixint, compress_varint, get_bit_length};

impl Compress for i128 {
    fn compress_multiple(objs: &[&Self], _opts: AutoCompressOpts) -> CompressedData {
        // Constant
        if objs.len() <= 1 {
            return CompressedData {
                engine: Engine::VarInt,
                binary_data: objs.iter().map(|x| compress_varint(**x)).collect(),
            };
        }

        // No compression
        let min = **objs.iter().min().unwrap();
        let max = **objs.iter().max().unwrap();

        let bit_length = get_bit_length((max - min) as u128);

        CompressedData {
            engine: Engine::FixedInt {
                bias: min,
                length: bit_length,
            },
            binary_data: objs
                .iter()
                .map(|num| compress_fixint((**num - min) as u128, bit_length))
                .collect(),
        }
    }

    fn split_categories(_objs: &[&Self]) -> Option<Vec<Vec<usize>>> {
        None
    }
}

macro_rules! impl_int {
    ($($t:ty),*) => {
        $(impl Compress for $t {
            fn compress_multiple(objs: &[&Self], opts: AutoCompressOpts) -> CompressedData {
                let nums: Vec<i128> = objs.iter().map(|x| **x as i128).collect();
                let objs: Vec<&i128> = nums.iter().collect();
                i128::compress_multiple(&objs, opts)
            }

            fn split_categories(_objs: &[&Self]) -> Option<Vec<Vec<usize>>> {
                None
            }
        })*
    }
}

impl_int!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, char);
