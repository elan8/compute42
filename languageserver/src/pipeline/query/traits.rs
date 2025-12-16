/// Trait for query operations
pub trait Query<T> {
    /// Execute the query and return results
    fn execute(&self) -> Result<T, crate::types::LspError>;
}


















