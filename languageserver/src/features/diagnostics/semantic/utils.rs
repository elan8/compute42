use crate::pipeline::storage::Index;
use std::collections::HashSet;

/// Check if a name is a Julia builtin, keyword, or Base function/type
pub(super) fn is_builtin_or_keyword(name: &str) -> bool {
    // Keywords
    if matches!(
        name,
        "if" | "else" | "elseif" | "end" | "for" | "while" | "function" | "return" | "break" | "continue"
            | "try" | "catch" | "finally" | "throw" | "let" | "begin" | "struct" | "abstract" | "type"
            | "mutable" | "module" | "using" | "import" | "export" | "const" | "global" | "local"
            | "true" | "false" | "nothing" | "missing" | "undef" | "Inf" | "NaN"
    ) {
        return true;
    }
    
    // Common Julia Base functions
    if matches!(
        name,
        // I/O and file operations
        "println" | "print" | "show" | "read" | "write" | "open" | "close" | "include" | "include_dependency"
        | "joinpath" | "abspath" | "basename" | "dirname" | "splitpath" | "normpath" | "mkpath" | "rm"
        | "isfile" | "isdir" | "exists" | "readdir" | "pwd" | "cd" | "homedir" | "tempdir" | "mktemp" | "mktempdir"
        // String operations
        | "string" | "repr" | "parse" | "tryparse" | "format" | "sprintf" | "length" | "size" | "first" | "last"
        | "getindex" | "setindex!" | "keys" | "values" | "pairs" | "iterate" | "collect" | "enumerate"
        // Collections
        | "Dict" | "Array" | "Vector" | "Matrix" | "Set" | "Tuple" | "NamedTuple" | "Pair"
        | "push!" | "pop!" | "append!" | "prepend!" | "insert!" | "delete!" | "empty!" | "filter" | "map"
        // Type system
        | "typeof" | "isa" | "supertype" | "subtypes" | "fieldnames" | "fieldcount" | "getfield" | "setfield!"
        | "Union" | "UnionAll" | "Type" | "DataType" | "AbstractType" | "Any" | "Nothing"
        // Numeric types
        | "Int" | "Int8" | "Int16" | "Int32" | "Int64" | "Int128" | "UInt" | "UInt8" | "UInt16" | "UInt32"
        | "UInt64" | "UInt128" | "Float16" | "Float32" | "Float64" | "BigInt" | "BigFloat" | "Rational" | "Complex"
        | "Bool" | "Char" | "String" | "Symbol"
        // Math operations
        | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "exp" | "log" | "log10"
        | "sqrt" | "abs" | "round" | "floor" | "ceil" | "trunc" | "mod" | "rem" | "div" | "fld" | "cld"
        | "+" | "-" | "*" | "/" | "\\" | "^" | "%" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "&&" | "||"
        // Statistics
        | "mean" | "median" | "std" | "var" | "sum" | "prod" | "maximum" | "minimum" | "extrema" | "cor" | "cov"
        // Utilities
        | "get" | "get!" | "haskey" | "in" | "∈" | "∉" | "isempty" | "isnothing" | "ismissing" | "something"
        | "zip" | "repeat" | "fill" | "zeros" | "ones" | "rand" | "randn" | "shuffle" | "sort" | "sort!"
        | "unique" | "unique!" | "count" | "findfirst" | "findlast" | "findall" | "findnext" | "findprev"
        | "eachindex" | "eachcol" | "eachrow" | "axes" | "ndims" | "eltype"
        | "argmax" | "argmin" | "skipmissing"
        // Special values and macros
        | "__DIR__" | "__FILE__" | "__LINE__" | "__MODULE__" | "@__DIR__" | "@__FILE__" | "@__LINE__" | "@__MODULE__"
        | "Missing" | "undef" | "nothing" | "missing"
    ) {
        return true;
    }
    
    false
}

/// Check if a module name is from Julia standard library
pub(super) fn is_stdlib_module(name: &str) -> bool {
    matches!(
        name,
        "Base" | "Core" | "Main" | "InteractiveUtils" | "REPL" | "Test" | "Random" | "Statistics"
            | "LinearAlgebra" | "SparseArrays" | "Distributed" | "SharedArrays" | "Dates" | "Printf"
            | "Downloads" | "Download" | "DataFrames" | "Plots" | "CSV"
            | "DelimitedFiles" | "Serialization" | "Profile" | "Pkg" | "Libdl" | "LibGit2"
    )
}

/// Find similar symbol using simple string distance
#[allow(dead_code)]
pub(super) fn find_similar_symbol(
    name: &str,
    defined_symbols: &HashSet<String>,
    _index: &Index,
) -> Option<String> {
    let mut best_match: Option<(String, usize)> = None;
    
    // Check locally defined symbols
    for symbol in defined_symbols {
        let distance = levenshtein_distance(name, symbol);
        if distance <= 2 && distance < name.len() / 2
            && (best_match.is_none() || best_match.as_ref().unwrap().1 > distance) {
            best_match = Some((symbol.clone(), distance));
        }
    }
    
    // Check symbol table (simplified - would need to iterate all symbols)
    // For now, we'll just return the best match from local symbols
    
    best_match.map(|(name, _)| name)
}

/// Simple Levenshtein distance implementation
#[allow(dead_code)]
pub(super) fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let n = s1_chars.len();
    let m = s2_chars.len();
    
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    
    let mut d = vec![vec![0; m + 1]; n + 1];
    
    for (i, row) in d.iter_mut().enumerate().take(n + 1) {
        row[0] = i;
    }
    for j in 0..=m {
        d[0][j] = j;
    }
    
    for i in 1..=n {
        for j in 1..=m {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
        }
    }
    
    d[n][m]
}


