use crate::varint::compress_varint;
use crate::compress::{Engine, Compress, CompressedData, AutoCompressOpts};

impl Compress for i128 {
    fn compress_multiple(objs: &[&Self], _opts: AutoCompressOpts) -> CompressedData {
        let avg = if objs.len() <= 3 {
            0
        } else {
            objs.iter().map(|x| **x).sum::<i128>() / (objs.len() as i128)
        };
        let engine = if avg == 0 {
            Engine::VarInt
        } else {
            Engine::BiasedVarInt { bias: avg }
        };
        CompressedData {
            engine,
            binary_data: objs.iter().map(|x| compress_varint(**x - avg)).collect(),
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
