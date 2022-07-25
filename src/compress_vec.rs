use crate::autocompress::{autocompress, AutoCompressOpts};
use crate::bits::Bits;
use crate::compress::{Compress, CompressedData, Engine};
use crate::split::try_split_by;
use crate::varint::{compress_fixint, get_bit_length};

impl<T: Compress> Compress for Vec<&T> {
    fn compress_multiple(objs: &[&Self], opts: AutoCompressOpts) -> CompressedData {
        // RLE
        let objs_rle: Vec<(Vec<usize>, Vec<&T>)> = objs
            .iter()
            .map(|vec| {
                let mut run_lengths: Vec<usize> = Vec::new();
                let mut run_values: Vec<&T> = Vec::new();
                let mut l = 0;
                while l < vec.len() {
                    let mut r = l;
                    while r < vec.len() && vec[r] == vec[l] {
                        r += 1;
                    }
                    run_lengths.push(r - l);
                    run_values.push(vec[l]);
                    l = r;
                }
                (run_lengths, run_values)
            })
            .collect();
        if objs_rle
            .iter()
            .map(|(run_lengths, _)| run_lengths.len())
            .sum::<usize>()
            < objs.iter().map(|vec| vec.len()).sum::<usize>() / 2
        {
            let run_lengths: Vec<&Vec<usize>> = objs_rle
                .iter()
                .map(|(run_lengths, _)| run_lengths)
                .collect();
            let run_values: Vec<&Vec<&T>> =
                objs_rle.iter().map(|(_, run_values)| run_values).collect();

            let run_lengths_compressed = autocompress(&run_lengths, AutoCompressOpts::default());
            let run_values_compressed = autocompress(&run_values, AutoCompressOpts::default());

            let mut binary_data = run_lengths_compressed.binary_data;
            for (i, bits) in binary_data.iter_mut().enumerate() {
                bits.extend(&run_values_compressed.binary_data[i]);
            }

            return CompressedData {
                engine: Engine::VecRLE {
                    length: Box::new(run_lengths_compressed.engine),
                    item: Box::new(run_values_compressed.engine),
                },
                binary_data,
            };
        }

        if let Some(data) = <T as EncodeVecSorted>::encode_vec_sorted(objs, opts) {
            return data;
        }

        // No compression
        encode_vec_raw(objs, opts)
    }

    fn split_categories(objs: &[&Self]) -> Option<Vec<Vec<usize>>> {
        // By length
        if let Some(categories) = try_split_by(objs, |vec| vec.len()) {
            return Some(categories);
        }
        // By item
        let min_length = objs.iter().map(|vec| vec.len()).min()?;
        for key in 0..min_length {
            if let Some(categories) = try_split_by(objs, |vec| &vec[key]) {
                return Some(categories);
            }
        }
        None
    }
}

impl<T: Compress> Compress for Vec<T> {
    default fn compress_multiple(objs: &[&Self], opts: AutoCompressOpts) -> CompressedData {
        let vecs: Vec<Vec<&T>> = objs.iter().map(|vec| vec.iter().collect()).collect();
        let refs: Vec<&Vec<&T>> = vecs.iter().collect();
        Vec::<&T>::compress_multiple(&refs, opts)
    }

    default fn split_categories(objs: &[&Self]) -> Option<Vec<Vec<usize>>> {
        let vecs: Vec<Vec<&T>> = objs.iter().map(|vec| vec.iter().collect()).collect();
        let refs: Vec<&Vec<&T>> = vecs.iter().collect();
        Vec::<&T>::split_categories(&refs)
    }
}

pub fn encode_vec_raw<T: Compress>(objs: &[&Vec<&T>], opts: AutoCompressOpts) -> CompressedData {
    let lengths: Vec<usize> = objs.iter().map(|vec| vec.len()).collect();
    let lengths_refs: Vec<&usize> = lengths.iter().collect();
    let lengths_compressed = autocompress(&lengths_refs, AutoCompressOpts::default());

    let items: Vec<&T> = objs.iter().map(|vec| *vec).flatten().map(|x| *x).collect();
    let items_compressed = autocompress(
        &items,
        AutoCompressOpts {
            enable_dedup_and_categories: opts.enable_dedup_and_categories,
            enable_stateful: false,
        },
    );

    let mut binary_data = Vec::with_capacity(objs.len());
    let mut offset = 0;

    for (i, mut bits) in lengths_compressed.binary_data.into_iter().enumerate() {
        for _ in 0..objs[i].len() {
            bits.extend(&items_compressed.binary_data[offset]);
            offset += 1;
        }
        binary_data.push(bits);
    }

    CompressedData {
        engine: Engine::Vec {
            length: Box::new(lengths_compressed.engine),
            item: Box::new(items_compressed.engine),
        },
        binary_data,
    }
}

pub trait EncodeVecSorted {
    fn encode_vec_sorted(objs: &[&Vec<&Self>], opts: AutoCompressOpts) -> Option<CompressedData>;
}

impl<T> EncodeVecSorted for T {
    default fn encode_vec_sorted(
        _objs: &[&Vec<&Self>],
        _opts: AutoCompressOpts,
    ) -> Option<CompressedData> {
        None
    }
}

macro_rules! impl_int {
    ($($t:ty),*) => {
        $(impl EncodeVecSorted for $t {
            fn encode_vec_sorted(objs: &[&Vec<&Self>], opts: AutoCompressOpts) -> Option<CompressedData> {
                if objs.is_empty() || objs.iter().all(|vec| vec.is_empty()) || !objs.iter().all(|vec| vec.windows(2).all(|window| window[0] <= window[1])) {
                    return None;
                }

                // Unique encoding only works correctly if there are at least two items in each set
                let unique = objs.iter().all(|vec| vec.len() >= 2 && vec.windows(2).all(|window| window[0] != window[1]));

                let min_elems: Vec<&Self> = objs.iter().filter_map(|vec| vec.first().cloned()).collect();
                let max_elems: Vec<&Self> = objs.iter().filter_map(|vec| vec.last().cloned()).collect();

                let min_elems_compressed = autocompress(&min_elems, opts);
                let max_elems_compressed = autocompress(&max_elems, opts);

                Some(CompressedData {
                    engine: Engine::IntSet {
                        min: Box::new(min_elems_compressed.engine),
                        max: Box::new(max_elems_compressed.engine),
                        unique,
                    },
                    binary_data: objs.iter().enumerate().map(|(i, vec)| {
                        let nums: Vec<i128> = vec.iter().map(|x| **x as i128).collect();
                        let mut bits = min_elems_compressed.binary_data[i].clone();
                        bits.extend(&max_elems_compressed.binary_data[i]);
                        if unique {
                            // As the min and max values are known, there's no need to list them
                            encode_ordered_set_slice(&nums[1..nums.len() - 1], nums[0] + 1, *nums.last().unwrap() - 1, true, &mut bits);
                        } else {
                            encode_ordered_set_slice(&nums, nums[0], *nums.last().unwrap(), false, &mut bits);
                        }
                        bits
                    }).collect(),
                })
            }
        })*
    }
}

impl_int!(u8); //, u16, u32, u64, usize, i8, i16, i32, i64, isize, char, i128);

fn encode_ordered_set_slice(slice: &[i128], min: i128, max: i128, unique: bool, bits: &mut Bits) {
    if slice.is_empty() {
        return;
    }

    let m = slice.len() / 2;

    if unique {
        let m_min = min + (m as i128);
        let m_max = max - (slice.len() - m - 1) as i128;
        let bit_length = get_bit_length((m_max - m_min) as u128);
        bits.extend(&compress_fixint((slice[m] - m_min) as u128, bit_length));
        encode_ordered_set_slice(&slice[..m], min, slice[m] - 1, true, bits);
        encode_ordered_set_slice(&slice[m + 1..], slice[m] + 1, max, true, bits);
    } else {
        let bit_length = get_bit_length((max - min) as u128);
        bits.extend(&compress_fixint((slice[m] - min) as u128, bit_length));
        encode_ordered_set_slice(&slice[..m], min, slice[m], false, bits);
        encode_ordered_set_slice(&slice[m + 1..], slice[m], max, false, bits);
    }
}
