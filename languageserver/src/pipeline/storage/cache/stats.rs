/// Cache key for documentation
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct DocsKey {
    pub symbol_name: String,
}

/// Cache key for hover info
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct HoverKey {
    pub file_uri: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub document_hits: u64,
    pub document_misses: u64,
    pub symbol_hits: u64,
    pub symbol_misses: u64,
    pub docs_hits: u64,
    pub docs_misses: u64,
    pub hover_hits: u64,
    pub hover_misses: u64,
}

impl CacheStats {
    /// Calculate overall hit rate
    pub fn hit_rate(&self) -> f64 {
        let total_hits = self.document_hits
            + self.symbol_hits
            + self.docs_hits
            + self.hover_hits;
        let total_requests = total_hits
            + self.document_misses
            + self.symbol_misses
            + self.docs_misses
            + self.hover_misses;
        
        if total_requests == 0 {
            0.0
        } else {
            (total_hits as f64) / (total_requests as f64)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CacheType {
    Document,
    Symbol,
    Docs,
    Hover,
}











