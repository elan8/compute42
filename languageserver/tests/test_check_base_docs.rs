/// Test to check if base_index.json contains the function signatures needed for test_demo_type_inference
use languageserver::pipeline::sources::BaseDocsRegistry;
use std::path::PathBuf;

#[test]
fn check_base_docs_for_test_functions() {
    // Find base_index.json
    // First try local temp directory (for testing), then fall back to user data directory
    let base_docs_path = if let Ok(env_path) = std::env::var("BASE_INDEX_OUTPUT_PATH") {
        PathBuf::from(env_path)
    } else {
        // Try local temp directory first
        let local_path = PathBuf::from("languageserver/temp/base_index.json");
        if local_path.exists() {
            local_path
        } else {
            // Fall back to user data directory
            let data_dir = dirs::data_local_dir()
                .map(|dir| dir.join("com.juliajunction.dev"))
                .unwrap_or_else(|| PathBuf::from("."));
            data_dir.join("base_index.json")
        }
    };
    
    if !base_docs_path.exists() {
        eprintln!("base_index.json not found at: {:?}", base_docs_path);
        eprintln!("Run the base extraction pipeline first!");
        eprintln!("Run: cargo test test_base_extraction -- --ignored");
        return;
    }
    
    println!("Loading base_index.json from: {:?}", base_docs_path);
    
    let registry = match BaseDocsRegistry::from_file(&base_docs_path) {
        Ok(reg) => reg,
        Err(e) => {
            eprintln!("Failed to load base_index.json: {:?}", e);
            return;
        }
    };
    
    println!("Loaded {} symbols from base_index.json\n", registry.len());
    
    // Functions needed by test_demo_type_inference.rs
    let needed_functions = vec![
        // Base functions
        "joinpath",
        "Base.joinpath",
        "Base.Filesystem.joinpath",
        "size",
        "Base.size",
        "length",
        "Base.length",
        "floor",
        "Base.floor",
        "unique",
        "Base.unique",
        // Statistics functions (might be in stdlib)
        "mean",
        "Statistics.mean",
        "std",
        "Statistics.std",
        "cor",
        "Statistics.cor",
        // Package functions (won't be in base_index.json)
        "CSV.read",
        "DataFrames.dropmissing",
    ];
    
    // Functions missing from hover output (from demo_hover_output.txt analysis)
    let missing_hover_functions = vec![
        "display",
        "Base.display",
        "string",
        "Base.string",
        "Base.String.string",
    ];
    
    println!("=== Checking for needed functions ===\n");
    
    let mut found_count = 0;
    let mut missing_count = 0;
    let mut found_functions = Vec::new();
    let mut missing_functions = Vec::new();
    
    // Check all functions (needed + missing from hover)
    let all_functions: Vec<_> = needed_functions.iter().chain(missing_hover_functions.iter()).collect();
    
    for func_name in &all_functions {
        let doc = registry.get_documentation(func_name);
        
        if doc.is_some() {
            found_count += 1;
            found_functions.push(func_name.to_string());
            println!("✓ Found: {}", func_name);
            if let Some(ref doc_str) = doc {
                let doc_preview: String = doc_str.chars().take(80).collect();
                println!("  Doc: {}", doc_preview);
            } else {
                println!("  Doc: (none)");
            }
        } else {
            missing_count += 1;
            missing_functions.push(func_name.to_string());
            println!("✗ Missing: {}", func_name);
        }
    }
    
    println!("\n=== Summary ===");
    println!("Found: {} / {}", found_count, all_functions.len());
    println!("Missing: {} / {}", missing_count, all_functions.len());
    
    if !found_functions.is_empty() {
        println!("\nFound functions:");
        for func in &found_functions {
            println!("  - {}", func);
        }
    }
    
    if !missing_functions.is_empty() {
        println!("\nMissing functions:");
        for func in &missing_functions {
            println!("  - {}", func);
        }
    }
    
    // Check if we have function signatures (not just return types)
    println!("\n=== Checking for function signature information ===");
    println!("Note: base_index.json now stores only doc strings (function name -> docstring).");
    println!("For type inference, we would need function signatures (parameter types + return types).");
    
    // Check a few Base functions to see what we have
    println!("\n=== Sample Base functions ===");
    let sample_functions = vec!["Base.size", "Base.length", "Base.joinpath"];
    for func_name in &sample_functions {
        if let Some(doc) = registry.get_documentation(func_name) {
            println!("\n{}:", func_name);
            println!("  Has doc: true");
            let doc_preview: String = doc.chars().take(80).collect();
            println!("  Doc preview: {}", doc_preview);
        } else {
            println!("\n{}: Not found", func_name);
        }
    }
    
    // Note about package functions
    println!("\n=== Note about package functions ===");
    println!("Package functions like CSV.read() and DataFrames.dropmissing() are NOT in base_index.json");
    println!("because base_index.json only contains Base/stdlib/Core functions.");
    println!("These would need to be indexed separately from package sources.");
    
    // Check Statistics module
    println!("\n=== Checking Statistics module ===");
    let stats_functions = vec!["Statistics.mean", "Statistics.std", "Statistics.cor"];
    for func_name in &stats_functions {
        if let Some(doc) = registry.get_documentation(func_name) {
            println!("✓ Found: {}", func_name);
            let doc_preview: String = doc.chars().take(80).collect();
            println!("  Doc preview: {}", doc_preview);
        } else {
            println!("✗ Missing: {}", func_name);
        }
    }
    
    // Check missing hover functions with detailed lookup attempts
    println!("\n=== Checking missing hover functions (display, string) ===");
    for func_name in &missing_hover_functions {
        println!("\nChecking: {}", func_name);
        if let Some(doc) = registry.get_documentation(func_name) {
            println!("  ✓ Found in registry");
            let doc_preview: String = doc.chars().take(100).collect();
            println!("  Doc preview: {}", doc_preview);
        } else {
            println!("  ✗ Not found in registry");
            // Try to find similar entries
            let similar = registry.find_entries_containing(func_name);
            if !similar.is_empty() {
                println!("  Similar entries found (first 5):");
                for entry in similar.iter().take(5) {
                    println!("    {}.{}", entry.module, entry.name);
                }
            }
        }
    }
    
    // Check what entries exist for "display" and "string"
    println!("\n=== Searching for entries containing 'display' or 'string' ===");
    let display_entries = registry.find_entries_containing("display");
    let string_entries = registry.find_entries_containing("string");
    println!("Entries containing 'display' (first 10):");
    for entry in display_entries.iter().take(10) {
        println!("  {}.{}", entry.module, entry.name);
    }
    println!("Entries containing 'string' (first 10):");
    for entry in string_entries.iter().take(10) {
        println!("  {}.{}", entry.module, entry.name);
    }
}






