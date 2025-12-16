use chrono::Local;

fn main() {
    // Set build date
    let build_date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
    println!("=== Compute42 Build Script Started ===");
    println!("Build date: {}", build_date);
    // Watch all Julia files for main and debugger functionality
    println!("cargo:rerun-if-changed=../internals/scripts/main.jl");
    println!("cargo:rerun-if-changed=../internals/scripts/core");
    println!("cargo:rerun-if-changed=../internals/scripts/debugger.jl");
    println!("cargo:rerun-if-changed=../internals/scripts/debugger");
    println!("cargo:rerun-if-changed=../../demo");

    tauri_build::build();
}
