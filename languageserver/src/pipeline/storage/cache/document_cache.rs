use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

/// Cache for parsed documents
pub struct DocumentCache {
    /// Cache storing document URIs to their last modified timestamp
    cache: Arc<RwLock<LruCache<String, u64>>>,
}

impl DocumentCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(capacity).unwrap())
            )),
        }
    }
    
    /// Check if document is cached and up-to-date
    pub fn is_valid(&self, uri: &str, timestamp: u64) -> bool {
        let cache = self.cache.read().unwrap();
        if let Some(&cached_timestamp) = cache.peek(uri) {
            cached_timestamp >= timestamp
        } else {
            false
        }
    }
    
    /// Update document cache
    pub fn update(&self, uri: String, timestamp: u64) {
        let mut cache = self.cache.write().unwrap();
        cache.put(uri, timestamp);
    }
    
    /// Invalidate document cache entry
    pub fn invalidate(&self, uri: &str) {
        let mut cache = self.cache.write().unwrap();
        cache.pop(uri);
    }
    
    /// Clear all entries
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_document_cache() {
        let cache = DocumentCache::new(10);
        
        // Initially not cached
        assert!(!cache.is_valid("file1.jl", 100));
        
        // Cache document
        cache.update("file1.jl".to_string(), 100);
        assert!(cache.is_valid("file1.jl", 100));
        assert!(cache.is_valid("file1.jl", 99)); // Older timestamp
        assert!(!cache.is_valid("file1.jl", 101)); // Newer timestamp
        
        // Invalidate
        cache.invalidate("file1.jl");
        assert!(!cache.is_valid("file1.jl", 100));
    }
}











