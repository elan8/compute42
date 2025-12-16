use crate::pipeline::sources::Document;
use crate::types::{Diagnostic, Range};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks document changes for incremental diagnostics
pub struct ChangeTracker {
    /// Map from file URI to last change time
    last_change: HashMap<String, Instant>,
    /// Map from file URI to last computed diagnostics version
    last_version: HashMap<String, i32>,
    /// Debounce delay in milliseconds
    debounce_delay: Duration,
}

impl ChangeTracker {
    /// Create a new change tracker with default debounce delay (300ms)
    pub fn new() -> Self {
        Self::with_debounce(Duration::from_millis(300))
    }
    
    /// Create a new change tracker with custom debounce delay
    pub fn with_debounce(debounce_delay: Duration) -> Self {
        Self {
            last_change: HashMap::new(),
            last_version: HashMap::new(),
            debounce_delay,
        }
    }
    
    /// Record a document change
    pub fn record_change(&mut self, uri: &str, version: i32) {
        self.last_change.insert(uri.to_string(), Instant::now());
        self.last_version.insert(uri.to_string(), version);
    }
    
    /// Check if diagnostics should be recomputed (debounced)
    pub fn should_recompute(&self, uri: &str) -> bool {
        if let Some(last_change_time) = self.last_change.get(uri) {
            last_change_time.elapsed() >= self.debounce_delay
        } else {
            true // First time, always recompute
        }
    }
    
    /// Check if document version has changed
    pub fn has_version_changed(&self, uri: &str, current_version: i32) -> bool {
        self.last_version
            .get(uri)
            .map(|&last| last != current_version)
            .unwrap_or(true)
    }
    
    /// Get affected ranges for incremental update
    /// This is a simplified implementation - in a full implementation,
    /// we'd track the actual edit ranges
    pub fn get_affected_ranges(&self, _uri: &str) -> Vec<Range> {
        // For now, return empty vec (full recomputation)
        // In a full implementation, we'd track edit ranges and return
        // the affected sections plus a buffer zone
        Vec::new()
    }
    
    /// Clear tracking for a file
    pub fn clear(&mut self, uri: &str) {
        self.last_change.remove(uri);
        self.last_version.remove(uri);
    }
    
    /// Clear all tracking
    pub fn clear_all(&mut self) {
        self.last_change.clear();
        self.last_version.clear();
    }
}

impl Default for ChangeTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Incremental diagnostics computer
pub struct IncrementalDiagnostics {
    change_tracker: ChangeTracker,
}

impl IncrementalDiagnostics {
    /// Create a new incremental diagnostics computer
    pub fn new() -> Self {
        Self {
            change_tracker: ChangeTracker::new(),
        }
    }
    
    /// Create with custom debounce delay
    pub fn with_debounce(debounce_delay: Duration) -> Self {
        Self {
            change_tracker: ChangeTracker::with_debounce(debounce_delay),
        }
    }
    
    /// Check if diagnostics should be recomputed for a document
    pub fn should_recompute(&self, document: &Document) -> bool {
        let uri = document.uri();
        let version = document.version();
        
        // Check if version changed
        if !self.change_tracker.has_version_changed(uri, version) {
            return false;
        }
        
        // Check debounce
        self.change_tracker.should_recompute(uri)
    }
    
    /// Record document change
    pub fn record_change(&mut self, document: &Document) {
        self.change_tracker.record_change(document.uri(), document.version());
    }
    
    /// Get affected ranges for incremental update
    pub fn get_affected_ranges(&self, document: &Document) -> Vec<Range> {
        self.change_tracker.get_affected_ranges(document.uri())
    }
    
    /// Clear tracking for a document
    pub fn clear(&mut self, uri: &str) {
        self.change_tracker.clear(uri);
    }
}

impl Default for IncrementalDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to compute diagnostics incrementally
/// For now, this is a placeholder - full incremental computation would
/// require tracking AST changes and only re-analyzing affected sections
pub fn compute_incremental(
    _document: &Document,
    previous_diagnostics: &[Diagnostic],
    affected_ranges: &[Range],
) -> Vec<Diagnostic> {
    // Simplified: if affected ranges are empty or cover the whole document,
    // do full recomputation. Otherwise, merge old and new diagnostics.
    
    if affected_ranges.is_empty() {
        // Full recomputation needed
        return Vec::new();
    }
    
    // Filter out diagnostics in affected ranges
    let kept_diagnostics: Vec<Diagnostic> = previous_diagnostics
        .iter()
        .filter(|d| {
            !affected_ranges.iter().any(|range| {
                ranges_overlap(&d.range, range)
            })
        })
        .cloned()
        .collect();
    
    // New diagnostics for affected ranges will be computed separately
    // and merged here. For now, we just return the kept ones.
    kept_diagnostics
}

/// Check if two ranges overlap
fn ranges_overlap(r1: &Range, r2: &Range) -> bool {
    !(r1.end.line < r2.start.line
        || r1.start.line > r2.end.line
        || (r1.end.line == r2.start.line && r1.end.character < r2.start.character)
        || (r1.start.line == r2.end.line && r1.start.character > r2.end.character))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Range, Position};
    
    #[test]
    fn test_change_tracker() {
        let mut tracker = ChangeTracker::new();
        
        tracker.record_change("test.jl", 1);
        assert!(tracker.has_version_changed("test.jl", 2));
        assert!(!tracker.has_version_changed("test.jl", 1));
    }
    
    #[test]
    fn test_ranges_overlap() {
        let r1 = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 5, character: 10 },
        };
        let r2 = Range {
            start: Position { line: 3, character: 0 },
            end: Position { line: 8, character: 10 },
        };
        assert!(ranges_overlap(&r1, &r2));
        
        let r3 = Range {
            start: Position { line: 10, character: 0 },
            end: Position { line: 15, character: 10 },
        };
        assert!(!ranges_overlap(&r1, &r3));
    }
}


