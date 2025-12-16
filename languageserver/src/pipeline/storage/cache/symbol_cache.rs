use crate::types::Symbol;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

/// Cache for symbol information (for hover)
pub struct SymbolCache {
    cache: Arc<RwLock<LruCache<String, Vec<Symbol>>>>,
}

impl SymbolCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(capacity).unwrap())
            )),
        }
    }
    
    /// Get cached symbols by name
    pub fn get(&self, symbol_name: &str) -> Option<Vec<Symbol>> {
        let mut cache = self.cache.write().unwrap();
        cache.get(symbol_name).cloned()
    }
    
    /// Cache symbols
    pub fn put(&self, symbol_name: String, symbols: Vec<Symbol>) {
        let mut cache = self.cache.write().unwrap();
        cache.put(symbol_name, symbols);
    }
    
    /// Invalidate symbol cache entry
    pub fn invalidate(&self, symbol_name: &str) {
        let mut cache = self.cache.write().unwrap();
        cache.pop(symbol_name);
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
    fn test_symbol_cache() {
        let cache = SymbolCache::new(10);
        
        // Initially empty
        assert!(cache.get("test_symbol").is_none());
        
        // Cache symbols
        let symbols = vec![];
        cache.put("test_symbol".to_string(), symbols.clone());
        assert_eq!(cache.get("test_symbol").unwrap(), symbols);
        
        // Invalidate
        cache.invalidate("test_symbol");
        assert!(cache.get("test_symbol").is_none());
    }
}











