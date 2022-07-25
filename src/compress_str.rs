use crate::autocompress::{autocompress, AutoCompressOpts};
use crate::compress::{Compress, CompressedData, Engine};
use crate::split::try_split_by;

impl Compress for String {
    fn compress_multiple(objs: &[&Self], opts: AutoCompressOpts) -> CompressedData {
        // Text separation
        for separator in ['\n', ' '] {
            if objs
                .iter()
                .map(|s| s.matches(separator).count())
                .sum::<usize>()
                >= objs.len()
            {
                let words: Vec<Vec<String>> = objs
                    .iter()
                    .map(|s| {
                        if s.is_empty() {
                            vec![]
                        } else {
                            s.split(separator).map(|word| word.to_string()).collect()
                        }
                    })
                    .collect();
                let words_refs: Vec<&Vec<String>> = words.iter().collect();
                let words_compressed = autocompress(&words_refs, opts);
                return CompressedData {
                    engine: Engine::StringConcat {
                        words: Box::new(words_compressed.engine),
                        separator,
                    },
                    binary_data: words_compressed.binary_data,
                };
            }
        }

        // Integers
        if objs.iter().all(|s| {
            if let Ok(n) = s.parse::<i128>() {
                n.to_string() == **s
            } else {
                false
            }
        }) {
            let nums: Vec<i128> = objs.iter().map(|s| s.parse::<i128>().unwrap()).collect();
            let nums_refs: Vec<&i128> = nums.iter().collect();
            let nums_compressed = autocompress(&nums_refs, opts);
            return CompressedData {
                engine: Engine::StringifiedInt {
                    inner: Box::new(nums_compressed.engine),
                },
                binary_data: nums_compressed.binary_data,
            };
        }

        // Decimals
        if objs.iter().all(|s| {
            if s.matches('.').count() == 1 {
                let s_sans_dot = s.replace(".", "");
                if let Ok(n) = s_sans_dot.parse::<i128>() {
                    return n.to_string() == s_sans_dot;
                }
            }
            false
        }) {
            let nums: Vec<i128> = objs
                .iter()
                .map(|s| s.replace(".", "").parse::<i128>().unwrap())
                .collect();
            let nums_refs: Vec<&i128> = nums.iter().collect();
            let nums_compressed = autocompress(&nums_refs, opts);

            let precisions: Vec<usize> = objs
                .iter()
                .map(|s| s.len() - s.find('.').unwrap() - 1)
                .collect();
            let precisions_refs: Vec<&usize> = precisions.iter().collect();
            let precisions_compressed = autocompress(&precisions_refs, opts);

            let mut binary_data = nums_compressed.binary_data;
            for (i, bits) in binary_data.iter_mut().enumerate() {
                bits.extend(&precisions_compressed.binary_data[i]);
            }

            return CompressedData {
                engine: Engine::StringifiedDecimal {
                    inner: Box::new(nums_compressed.engine),
                    precision: Box::new(precisions_compressed.engine),
                },
                binary_data,
            };
        }

        let chars: Vec<Vec<char>> = objs.iter().map(|s| s.chars().collect()).collect();
        let chars_refs: Vec<&Vec<char>> = chars.iter().collect();
        let compressed = autocompress(&chars_refs, opts);

        CompressedData {
            engine: Engine::String {
                chars: Box::new(compressed.engine),
            },
            binary_data: compressed.binary_data,
        }
    }

    fn split_categories(objs: &[&Self]) -> Option<Vec<Vec<usize>>> {
        // By kind
        if let Some(categories) = try_split_by(objs, |s| get_string_kind(s)) {
            return Some(categories);
        }
        None
    }
}

#[derive(PartialEq, Eq, Hash)]
enum StringKind {
    Empty,
    DecimalNumber,
    ExtendedDecimalNumber,
    Latin,
    LatinNumeric,
    Text,
    Generic,
}

fn get_string_kind(s: &str) -> StringKind {
    if s.is_empty() {
        return StringKind::Empty;
    } else if s.chars().all(|c| c.is_digit(10)) {
        return StringKind::DecimalNumber;
    } else if s
        .chars()
        .all(|c| c == '+' || c == '-' || c == '.' || c == 'e' || c == 'E' || c.is_digit(10))
    {
        return StringKind::ExtendedDecimalNumber;
    } else if s
        .chars()
        .all(|c| ('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z'))
    {
        return StringKind::Latin;
    } else if s
        .chars()
        .all(|c| ('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z') || c.is_digit(10))
    {
        return StringKind::LatinNumeric;
    } else if s.chars().all(|c| (c as u32) < 128) {
        return StringKind::Text;
    } else {
        return StringKind::Generic;
    }
}
