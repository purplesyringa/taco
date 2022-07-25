use crate::compress::{Engine, Compress, CompressedData, autocompress, AutoCompressOpts};
use crate::split::try_split_by;

impl<T: Compress> Compress for Vec<T> {
    fn compress_multiple(objs: &[&Self], opts: AutoCompressOpts) -> CompressedData {
        let lengths: Vec<usize> = objs.iter().map(|vec| vec.len()).collect();
        let lengths_refs: Vec<&usize> = lengths.iter().collect();
        let lengths_compressed = autocompress(&lengths_refs, AutoCompressOpts::default());

        let items: Vec<&T> = objs.iter().map(|vec| *vec).flatten().collect();
        let items_compressed = autocompress(&items, opts);

        CompressedData {
            engine: Engine::Vec {
                length: Box::new(lengths_compressed.engine),
                item: Box::new(items_compressed.engine),
            },
            binary_data: lengths_compressed.binary_data.into_iter().enumerate().map(|(i, mut bits)| {
                bits.extend(&items_compressed.binary_data[i]);
                bits
            }).collect()
        }
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
