use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Represents a Julia type expression
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeExpr {
    /// Concrete type like "DataFrame", "Int64"
    Concrete(String),
    /// Union type like Union{Int64, Missing}
    Union(Vec<TypeExpr>),
    /// Generic type like Vector{Int64}, Dict{String, Int64}
    Generic(String, Vec<TypeExpr>),
    /// Any type
    Any,
    /// Unknown type
    Unknown,
}

impl TypeExpr {
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            TypeExpr::Concrete(name) => name.clone(),
            TypeExpr::Union(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                format!("Union{{{}}}", type_strs.join(", "))
            }
            TypeExpr::Generic(name, params) => {
                let param_strs: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                format!("{}{{{}}}", name, param_strs.join(", "))
            }
            TypeExpr::Any => "Any".to_string(),
            TypeExpr::Unknown => "Unknown".to_string(),
        }
    }
}

/// Represents a function parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<TypeExpr>,
}

/// Represents a function signature with return type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub module: String,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeExpr>,
    pub doc_comment: Option<String>,
    /// File URI where this signature is defined
    pub file_uri: String,
    /// Range in the source file
    pub range: crate::types::Range,
}

/// Represents a type definition (struct, abstract type, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub module: String,
    pub name: String,
    pub kind: TypeDefinitionKind,
    pub doc_comment: Option<String>,
    /// File URI where this type is defined
    pub file_uri: String,
    /// Range in the source file
    pub range: crate::types::Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TypeDefinitionKind {
    Struct,
    Abstract,
    Primitive,
    Union,
}

/// Represents a DataFrame schema with column names and types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataFrameSchema {
    pub columns: HashMap<String, TypeExpr>,
}

impl DataFrameSchema {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }

    pub fn add_column(&mut self, name: String, column_type: TypeExpr) {
        self.columns.insert(name, column_type);
    }

    pub fn get_column_type(&self, name: &str) -> Option<&TypeExpr> {
        self.columns.get(name)
    }
}

impl Default for DataFrameSchema {
    fn default() -> Self {
        Self::new()
    }
}
