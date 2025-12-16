mod document_cache;
mod symbol_cache;
mod docs_cache;
mod hover_cache;
mod file_type_map;
mod stats;
mod diagnostics_cache;

pub use document_cache::DocumentCache;
pub use symbol_cache::SymbolCache;
pub use docs_cache::DocsCache;
pub use hover_cache::HoverCache;
pub use file_type_map::FileTypeMapCache;
pub use diagnostics_cache::DiagnosticsCache;
pub use stats::{CacheStats, CacheType, DocsKey, HoverKey};

use std::sync::{Arc, RwLock};

/// Central cache manager that coordinates all caches
pub struct CacheManager {
    pub document_cache: DocumentCache,
    pub symbol_cache: SymbolCache,
    pub docs_cache: DocsCache,
    pub hover_cache: HoverCache,
    pub file_type_map: FileTypeMapCache,
    pub diagnostics_cache: DiagnosticsCache,
    
    /// Statistics for cache performance
    stats: Arc<RwLock<CacheStats>>,
}

impl CacheManager {
    /// Create a new cache manager with default capacities
    pub fn new() -> Self {
        Self::with_capacities(1000, 5000, 2000, 500, 1000)
    }
    
    /// Create a cache manager with custom capacities
    pub fn with_capacities(
        document_capacity: usize,
        symbol_capacity: usize,
        docs_capacity: usize,
        hover_capacity: usize,
        diagnostics_capacity: usize,
    ) -> Self {
        log::trace!(
            "CacheManager: Creating with capacities - doc: {}, sym: {}, docs: {}, hover: {}, diagnostics: {}",
            document_capacity,
            symbol_capacity,
            docs_capacity,
            hover_capacity,
            diagnostics_capacity
        );
        
        Self {
            document_cache: DocumentCache::new(document_capacity),
            symbol_cache: SymbolCache::new(symbol_capacity),
            docs_cache: DocsCache::new(docs_capacity),
            hover_cache: HoverCache::new(hover_capacity),
            file_type_map: FileTypeMapCache::new(256),
            diagnostics_cache: DiagnosticsCache::with_capacity(diagnostics_capacity),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }
    
    /// Invalidate all caches for a specific file
    pub fn invalidate_file(&self, file_uri: &str) {
        log::trace!("CacheManager: Invalidating caches for file: {}", file_uri);
        self.document_cache.invalidate(file_uri);
        self.hover_cache.invalidate_file(file_uri);
        self.diagnostics_cache.invalidate(file_uri);
        // Symbol cache and docs cache are global, so we don't invalidate them
    }
    
    /// Clear all caches
    pub fn clear_all(&self) {
        log::trace!("CacheManager: Clearing all caches");
        self.document_cache.clear();
        self.symbol_cache.clear();
        self.docs_cache.clear();
        self.hover_cache.clear();
        self.file_type_map.clear();
        self.diagnostics_cache.clear();
    }
    
    /// Record a cache hit
    pub fn record_hit(&self, cache_type: CacheType) {
        let mut stats = self.stats.write().unwrap();
        match cache_type {
            CacheType::Document => stats.document_hits += 1,
            CacheType::Symbol => stats.symbol_hits += 1,
            CacheType::Docs => stats.docs_hits += 1,
            CacheType::Hover => stats.hover_hits += 1,
        }
    }
    
    /// Record a cache miss
    pub fn record_miss(&self, cache_type: CacheType) {
        let mut stats = self.stats.write().unwrap();
        match cache_type {
            CacheType::Document => stats.document_misses += 1,
            CacheType::Symbol => stats.symbol_misses += 1,
            CacheType::Docs => stats.docs_misses += 1,
            CacheType::Hover => stats.hover_misses += 1,
        }
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.read().unwrap().clone()
    }
    
    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write().unwrap();
        *stats = CacheStats::default();
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_manager() {
        let manager = CacheManager::new();
        
        // Test document cache via manager
        manager.document_cache.update("file1.jl".to_string(), 100);
        assert!(manager.document_cache.is_valid("file1.jl", 100));
        
        // Test file invalidation
        manager.invalidate_file("file1.jl");
        assert!(!manager.document_cache.is_valid("file1.jl", 100));
        
        // Test clear all
        manager.document_cache.update("file2.jl".to_string(), 200);
        manager.clear_all();
        assert!(!manager.document_cache.is_valid("file2.jl", 200));
    }
    
    #[test]
    fn test_cache_stats() {
        let manager = CacheManager::new();
        
        // Record some hits and misses
        manager.record_hit(CacheType::Document);
        manager.record_hit(CacheType::Symbol);
        manager.record_miss(CacheType::Docs);
        
        let stats = manager.stats();
        assert_eq!(stats.document_hits, 1);
        assert_eq!(stats.symbol_hits, 1);
        assert_eq!(stats.docs_misses, 1);
        
        // Hit rate should be 2/3 = 0.666...
        let hit_rate = stats.hit_rate();
        assert!((hit_rate - 0.666).abs() < 0.01);
        
        // Reset stats
        manager.reset_stats();
        let stats = manager.stats();
        assert_eq!(stats.document_hits, 0);
        assert_eq!(stats.hit_rate(), 0.0);
    }
}

