use std::hash::Hash;
use std::collections::HashMap;
use crate::compress::Compress;

pub fn try_split_by<'a, T: Compress, K: Hash + Eq>(objs: &[&'a T], key_fn: impl Fn(&'a T) -> K) -> Option<Vec<Vec<usize>>> {
    let mut indices_by_value: HashMap<K, Vec<usize>> = HashMap::new();
    for (i, x) in objs.iter().enumerate() {
        indices_by_value.entry(key_fn(x)).or_default().push(i);
    }
    if indices_by_value.len() < objs.len() / 2 {
        Some(indices_by_value.into_iter().map(|(_, v)| v).collect())
    } else {
        None
    }
}
