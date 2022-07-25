use crate::compress::{Engine, Compress, CompressedData, autocompress, AutoCompressOpts};
use crate::split::try_split_by;

impl Compress for String {
    fn compress_multiple(objs: &[&Self], opts: AutoCompressOpts) -> CompressedData {
        for separator in ['\n', ' '] {
            if objs.iter().map(|s| s.matches(separator).count()).sum::<usize>() >= objs.len() {
                let words: Vec<Vec<String>> = objs.iter().map(|s| s.split(separator).map(|word| word.to_string()).collect()).collect();
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
    } else if s.chars().all(|c| c == '+' || c == '-' || c == '.' || c == 'e' || c == 'E' || c.is_digit(10)) {
        return StringKind::ExtendedDecimalNumber;
    } else if s.chars().all(|c| ('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z')) {
        return StringKind::Latin;
    } else if s.chars().all(|c| ('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z') || c.is_digit(10)) {
        return StringKind::LatinNumeric;
    } else if s.chars().all(|c| (c as u32) < 128) {
        return StringKind::Text;
    } else {
        return StringKind::Generic;
    }
}
