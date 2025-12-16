use super::stats::DocsKey;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

/// Cache for Julia documentation
pub struct DocsCache {
    cache: Arc<RwLock<LruCache<DocsKey, Option<String>>>>,
}

impl DocsCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(capacity).unwrap())
            )),
        }
    }
    
    /// Get cached documentation
    pub fn get(&self, symbol_name: &str) -> Option<Option<String>> {
        let key = DocsKey {
            symbol_name: symbol_name.to_string(),
        };
        let mut cache = self.cache.write().unwrap();
        cache.get(&key).cloned()
    }
    
    /// Cache documentation
    pub fn put(&self, symbol_name: String, docs: Option<String>) {
        let key = DocsKey { symbol_name };
        let mut cache = self.cache.write().unwrap();
        cache.put(key, docs);
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
    fn test_docs_cache() {
        let cache = DocsCache::new(10);
        
        // Initially empty
        assert!(cache.get("println").is_none());
        
        // Cache docs
        cache.put("println".to_string(), Some("Print to stdout".to_string()));
        assert_eq!(
            cache.get("println").unwrap(),
            Some("Print to stdout".to_string())
        );
    }
}











