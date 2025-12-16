use crate::types::Diagnostic;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Cache for diagnostic results
pub struct DiagnosticsCache {
    /// Map from (file_uri, version) to diagnostics
    cache: Arc<RwLock<HashMap<String, (i32, Vec<Diagnostic>)>>>,
    capacity: usize,
}

impl DiagnosticsCache {
    /// Create a new diagnostics cache with default capacity
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }
    
    /// Create a new diagnostics cache with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
            capacity,
        }
    }
    
    /// Get cached diagnostics for a file
    pub fn get(&self, file_uri: &str, version: i32) -> Option<Vec<Diagnostic>> {
        let cache = self.cache.read().ok()?;
        if let Some((cached_version, diagnostics)) = cache.get(file_uri) {
            if *cached_version == version {
                return Some(diagnostics.clone());
            }
        }
        None
    }
    
    /// Store diagnostics for a file
    pub fn put(&self, file_uri: &str, version: i32, diagnostics: Vec<Diagnostic>) {
        let mut cache = self.cache.write().unwrap();
        
        // Evict if cache is too large
        if cache.len() >= self.capacity {
            // Remove oldest entries (simple strategy: remove first)
            if let Some(key) = cache.keys().next().cloned() {
                cache.remove(&key);
            }
        }
        
        cache.insert(file_uri.to_string(), (version, diagnostics));
    }
    
    /// Invalidate cache for a specific file
    pub fn invalidate(&self, file_uri: &str) {
        let mut cache = self.cache.write().unwrap();
        cache.remove(file_uri);
    }
    
    /// Invalidate cache for all files
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }
    
    /// Check if cache has entry for file (regardless of version)
    pub fn has_entry(&self, file_uri: &str) -> bool {
        let cache = self.cache.read().unwrap();
        cache.contains_key(file_uri)
    }
    
    /// Get cache size
    pub fn len(&self) -> usize {
        let cache = self.cache.read().unwrap();
        cache.len()
    }
    
    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        let cache = self.cache.read().unwrap();
        cache.is_empty()
    }
}

impl Default for DiagnosticsCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Diagnostic, DiagnosticSeverity, Range, Position};
    
    fn create_test_diagnostic() -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 5 },
            },
            severity: Some(DiagnosticSeverity::Error),
            code: Some("test".to_string()),
            source: Some("test".to_string()),
            message: "Test diagnostic".to_string(),
            related_information: None,
        }
    }
    
    #[test]
    fn test_cache_get_put() {
        let cache = DiagnosticsCache::new();
        let diagnostics = vec![create_test_diagnostic()];
        
        cache.put("test.jl", 1, diagnostics.clone());
        
        let cached = cache.get("test.jl", 1);
        assert!(cached.is_some());
        assert_eq!(cached.as_ref().unwrap().len(), diagnostics.len());
        assert!(cache.get("test.jl", 2).is_none()); // Different version
    }
    
    #[test]
    fn test_cache_invalidate() {
        let cache = DiagnosticsCache::new();
        let diagnostics = vec![create_test_diagnostic()];
        
        cache.put("test.jl", 1, diagnostics);
        cache.invalidate("test.jl");
        
        assert!(cache.get("test.jl", 1).is_none());
    }
    
    #[test]
    fn test_cache_clear() {
        let cache = DiagnosticsCache::new();
        let diagnostics = vec![create_test_diagnostic()];
        
        cache.put("test1.jl", 1, diagnostics.clone());
        cache.put("test2.jl", 1, diagnostics);
        cache.clear();
        
        assert!(cache.is_empty());
    }
}



