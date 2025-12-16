use serde::{Serialize, Deserialize};
use crate::types::type_expr::TypeExpr;

/// Symbol information stored in the base docs file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolInfo {
    /// Documentation string for the symbol
    pub doc: Option<String>,
    /// Full function signatures (for functions with multiple dispatch)
    /// This includes parameter types and return types needed for type inference
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signatures: Vec<FunctionSignatureInfo>,
}

/// Simplified function signature info for storage in base_index.json
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSignatureInfo {
    /// Function parameters with their types
    pub parameters: Vec<ParameterInfo>,
    /// Return type of the function
    pub return_type: Option<TypeExpr>,
    /// Documentation for this specific signature
    pub doc: Option<String>,
}

/// Parameter information for function signatures
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterInfo {
    /// Parameter name
    pub name: String,
    /// Parameter type (if known)
    pub param_type: Option<TypeExpr>,
}
