use crate::types::HoverResult;
use super::stats::HoverKey;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

/// Cache for hover results
pub struct HoverCache {
    cache: Arc<RwLock<LruCache<HoverKey, HoverResult>>>,
}

impl HoverCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(capacity).unwrap())
            )),
        }
    }
    
    /// Get cached hover result
    pub fn get(&self, file_uri: &str, line: u32, character: u32) -> Option<HoverResult> {
        let key = HoverKey {
            file_uri: file_uri.to_string(),
            line,
            character,
        };
        let mut cache = self.cache.write().unwrap();
        cache.get(&key).cloned()
    }
    
    /// Cache hover result
    pub fn put(&self, file_uri: String, line: u32, character: u32, result: HoverResult) {
        let key = HoverKey {
            file_uri,
            line,
            character,
        };
        let mut cache = self.cache.write().unwrap();
        cache.put(key, result);
    }
    
    /// Invalidate hover cache for a specific file
    pub fn invalidate_file(&self, file_uri: &str) {
        let mut cache = self.cache.write().unwrap();
        // Remove all entries for this file
        let keys_to_remove: Vec<HoverKey> = cache
            .iter()
            .filter(|(k, _)| k.file_uri == file_uri)
            .map(|(k, _)| k.clone())
            .collect();
        
        for key in keys_to_remove {
            cache.pop(&key);
        }
    }
    
    /// Clear all entries
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }
}

