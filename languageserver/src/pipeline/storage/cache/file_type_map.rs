use crate::types::{Position, Range};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

/// Cached file-level type map.
/// Stores a list of (range, type_string) per file.
#[derive(Clone)]
pub struct FileTypeMapCache {
    cache: Arc<RwLock<LruCache<String, Vec<(Range, String)>>>>,
}

impl FileTypeMapCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(capacity).unwrap())
            )),
        }
    }

    pub fn put_file_types(&self, file_uri: String, entries: Vec<(Range, String)>) {
        let mut cache = self.cache.write().unwrap();
        cache.put(file_uri, entries);
    }

    /// Return all type strings whose ranges contain the given position
    pub fn types_at(&self, file_uri: &str, pos: Position) -> Vec<String> {
        let mut cache = self.cache.write().unwrap();
        if let Some(entries) = cache.get(&file_uri.to_string()) {
            let mut out = Vec::new();
            for (r, t) in entries.iter() {
                if Self::contains(r, pos) { out.push(t.clone()); }
            }
            out
        } else {
            Vec::new()
        }
    }

    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    fn contains(r: &Range, p: Position) -> bool {
        p.line >= r.start.line && p.line <= r.end.line &&
        (p.line > r.start.line || p.character >= r.start.character) &&
        (p.line < r.end.line || p.character <= r.end.character)
    }
}











