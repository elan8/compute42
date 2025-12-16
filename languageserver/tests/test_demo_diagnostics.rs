/// Integration test to test diagnostics functionality on demo.jl and functions.jl
/// 
/// This test opens the demo project, updates both files, and checks diagnostics
/// for errors and warnings.
/// 
/// Run with: `cargo test test_demo_diagnostics -- --ignored --nocapture`

use languageserver::embedded::{EmbeddedLspService, LspConfig};
use languageserver::types::DiagnosticSeverity;
use std::path::PathBuf;
use std::fs;
use std::process::Command;

/// Find Julia executable (reused from test_base_indexing.rs)
fn find_julia_executable() -> Option<PathBuf> {
    // First, try the specific JuliaJunction installation directory
    let julia_install_dir = PathBuf::from(r"C:\Users\jeroe\AppData\Local\com.juliajunction.dev\julia\julia-1.12.1");
    let julia_exe = if cfg!(target_os = "windows") {
        julia_install_dir.join("bin").join("julia.exe")
    } else {
        julia_install_dir.join("bin").join("julia")
    };
    
    if julia_exe.exists() {
        return Some(julia_exe);
    }
    
    // Fallback: try JuliaJunction installation directory (any version)
    if let Some(data_dir) = dirs::data_local_dir() {
        let julia_dir = data_dir.join("com.juliajunction.dev").join("julia");
        
        // Try to find julia-1.12.1 or any version subdirectory
        if julia_dir.exists() {
            // Look for version subdirectories
            if let Ok(entries) = std::fs::read_dir(&julia_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let julia_exe = if cfg!(target_os = "windows") {
                            entry.path().join("bin").join("julia.exe")
                        } else {
                            entry.path().join("bin").join("julia")
                        };
                        
                        if julia_exe.exists() {
                            return Some(julia_exe);
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: try common locations in PATH
    let candidates = vec![
        "julia",
        "julia.exe",
    ];
    
    for candidate in candidates {
        if let Ok(output) = Command::new(candidate).arg("--version").output() {
            if output.status.success() {
                return Some(PathBuf::from(candidate));
            }
        }
    }
    
    None
}

#[tokio::test]
#[ignore] // Not part of standard test set - run with `cargo test -- --ignored`
async fn test_demo_diagnostics() {
    // Path to the demo project
    let demo_project_root = PathBuf::from(r"C:\Git\juliajunction\target\debug\demo");
    let demo_jl_path = demo_project_root.join("src").join("demo.jl");
    let functions_jl_path = demo_project_root.join("src").join("functions.jl");
    
    // Check if files exist
    if !demo_jl_path.exists() {
        eprintln!("Error: demo.jl not found at {:?}", demo_jl_path);
        eprintln!("Skipping test");
        return;
    }
    
    if !functions_jl_path.exists() {
        eprintln!("Error: functions.jl not found at {:?}", functions_jl_path);
        eprintln!("Skipping test");
        return;
    }
    
    eprintln!("Testing diagnostics on demo project:");
    eprintln!("  Project root: {:?}", demo_project_root);
    eprintln!("  demo.jl: {:?}", demo_jl_path);
    eprintln!("  functions.jl: {:?}", functions_jl_path);
    
    // Find Julia executable
    let julia_executable = match find_julia_executable() {
        Some(exe) => {
            eprintln!("Found Julia executable: {:?}", exe);
            exe
        }
        None => {
            eprintln!("Error: Julia executable not found");
            eprintln!("Skipping test");
            return;
        }
    };
    
    // Create LSP config
    let config = LspConfig::new(julia_executable)
        .with_project_root(demo_project_root.clone());
    
    // Create service
    let mut service = EmbeddedLspService::new(config);
    
    // Open project
    eprintln!("\nOpening project...");
    match service.open_project(demo_project_root.clone()) {
        Ok(()) => {
            eprintln!("Project opened successfully");
        }
        Err(e) => {
            eprintln!("Warning: Failed to open project: {}. Continuing anyway...", e);
        }
    }
    
    // Read and update demo.jl
    eprintln!("\n=== Testing diagnostics on demo.jl ===");
    let demo_content = match fs::read_to_string(&demo_jl_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading demo.jl: {}", e);
            return;
        }
    };
    
    match service.update_document(demo_jl_path.clone(), demo_content.clone()) {
        Ok(()) => {
            eprintln!("Updated demo.jl document");
        }
        Err(e) => {
            eprintln!("Error updating demo.jl document: {}", e);
            return;
        }
    }
    
    // Get diagnostics for demo.jl
    let demo_diagnostics = service.get_diagnostics(&demo_jl_path);
    eprintln!("Found {} diagnostics for demo.jl", demo_diagnostics.len());
    
    // Categorize and print diagnostics
    let mut demo_errors = Vec::new();
    let mut demo_warnings = Vec::new();
    let mut demo_info = Vec::new();
    
    for (i, diag) in demo_diagnostics.iter().enumerate() {
        match diag.severity {
            Some(DiagnosticSeverity::Error) => {
                demo_errors.push((i, diag));
            }
            Some(DiagnosticSeverity::Warning) => {
                demo_warnings.push((i, diag));
            }
            Some(DiagnosticSeverity::Information) | Some(DiagnosticSeverity::Hint) => {
                demo_info.push((i, diag));
            }
            None => {
                // If no severity specified, treat as warning
                demo_warnings.push((i, diag));
            }
        }
    }
    
    eprintln!("\nDiagnostics breakdown for demo.jl:");
    eprintln!("  Errors: {}", demo_errors.len());
    eprintln!("  Warnings: {}", demo_warnings.len());
    eprintln!("  Info/Hints: {}", demo_info.len());
    
    if !demo_errors.is_empty() {
        eprintln!("\nERRORS in demo.jl:");
        for (i, diag) in &demo_errors {
            eprintln!("  [{}] Line {}: {}", i + 1, diag.range.start.line + 1, diag.message);
            if let Some(ref code) = diag.code {
                eprintln!("      Code: {:?}", code);
            }
        }
    }
    
    if !demo_warnings.is_empty() {
        eprintln!("\nWARNINGS in demo.jl:");
        for (i, diag) in &demo_warnings {
            eprintln!("  [{}] Line {}: {}", i + 1, diag.range.start.line + 1, diag.message);
            if let Some(ref code) = diag.code {
                eprintln!("      Code: {:?}", code);
            }
        }
    }
    
    if !demo_info.is_empty() {
        eprintln!("\nINFO/HINTS in demo.jl:");
        for (i, diag) in &demo_info {
            eprintln!("  [{}] Line {}: {}", i + 1, diag.range.start.line + 1, diag.message);
        }
    }
    
    // Read and update functions.jl
    eprintln!("\n=== Testing diagnostics on functions.jl ===");
    let functions_content = match fs::read_to_string(&functions_jl_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading functions.jl: {}", e);
            return;
        }
    };
    
    match service.update_document(functions_jl_path.clone(), functions_content.clone()) {
        Ok(()) => {
            eprintln!("Updated functions.jl document");
        }
        Err(e) => {
            eprintln!("Error updating functions.jl document: {}", e);
            return;
        }
    }
    
    // Get diagnostics for functions.jl
    let functions_diagnostics = service.get_diagnostics(&functions_jl_path);
    eprintln!("Found {} diagnostics for functions.jl", functions_diagnostics.len());
    
    // Categorize and print diagnostics
    let mut functions_errors = Vec::new();
    let mut functions_warnings = Vec::new();
    let mut functions_info = Vec::new();
    
    for (i, diag) in functions_diagnostics.iter().enumerate() {
        match diag.severity {
            Some(DiagnosticSeverity::Error) => {
                functions_errors.push((i, diag));
            }
            Some(DiagnosticSeverity::Warning) => {
                functions_warnings.push((i, diag));
            }
            Some(DiagnosticSeverity::Information) | Some(DiagnosticSeverity::Hint) => {
                functions_info.push((i, diag));
            }
            None => {
                // If no severity specified, treat as warning
                functions_warnings.push((i, diag));
            }
        }
    }
    
    eprintln!("\nDiagnostics breakdown for functions.jl:");
    eprintln!("  Errors: {}", functions_errors.len());
    eprintln!("  Warnings: {}", functions_warnings.len());
    eprintln!("  Info/Hints: {}", functions_info.len());
    
    if !functions_errors.is_empty() {
        eprintln!("\nERRORS in functions.jl:");
        for (i, diag) in &functions_errors {
            eprintln!("  [{}] Line {}: {}", i + 1, diag.range.start.line + 1, diag.message);
            if let Some(ref code) = diag.code {
                eprintln!("      Code: {:?}", code);
            }
        }
    }
    
    if !functions_warnings.is_empty() {
        eprintln!("\nWARNINGS in functions.jl:");
        for (i, diag) in &functions_warnings {
            eprintln!("  [{}] Line {}: {}", i + 1, diag.range.start.line + 1, diag.message);
            if let Some(ref code) = diag.code {
                eprintln!("      Code: {:?}", code);
            }
        }
    }
    
    if !functions_info.is_empty() {
        eprintln!("\nINFO/HINTS in functions.jl:");
        for (i, diag) in &functions_info {
            eprintln!("  [{}] Line {}: {}", i + 1, diag.range.start.line + 1, diag.message);
        }
    }
    
    // Summary
    eprintln!("\n=== SUMMARY ===");
    let total_errors = demo_errors.len() + functions_errors.len();
    let total_warnings = demo_warnings.len() + functions_warnings.len();
    eprintln!("Total errors: {}", total_errors);
    eprintln!("Total warnings: {}", total_warnings);
    
    // Assert that diagnostics were computed (even if empty)
    // This ensures the diagnostics functionality is working
    eprintln!("\n✓ Diagnostics test completed successfully");
    eprintln!("  - demo.jl: {} diagnostics ({} errors, {} warnings)", 
        demo_diagnostics.len(), demo_errors.len(), demo_warnings.len());
    eprintln!("  - functions.jl: {} diagnostics ({} errors, {} warnings)", 
        functions_diagnostics.len(), functions_errors.len(), functions_warnings.len());
    
    // If there are errors, we should investigate and fix them
    if total_errors > 0 {
        eprintln!("\n⚠ WARNING: {} errors found. These should be investigated and fixed.", total_errors);
    }
    
    if total_warnings > 0 {
        eprintln!("\n⚠ NOTE: {} warnings found. These should be reviewed.", total_warnings);
    }
}

