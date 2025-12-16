use crate::types::LspError;

/// Trait that all pipelines must implement
/// 
/// This trait provides a common interface for all pipeline types, allowing them
/// to be used polymorphically while maintaining type safety through associated types.
pub trait Pipeline {
    /// The input type for this pipeline
    type Input;
    
    /// The output type for this pipeline
    type Output;
    
    /// Run the pipeline with the given input
    /// 
    /// This is the primary method that executes the pipeline's main functionality.
    /// Each pipeline implementation defines what input it accepts and what output it produces.
    fn run(&self, input: Self::Input) -> Result<Self::Output, LspError>;
    
    /// Get the name of this pipeline
    /// 
    /// Returns a human-readable name identifying the pipeline type.
    fn name(&self) -> &'static str;
    
    /// Get a description of what this pipeline does
    /// 
    /// Returns a human-readable description of the pipeline's purpose and functionality.
    fn description(&self) -> &'static str;
}



