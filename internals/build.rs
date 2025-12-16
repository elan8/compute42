use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=scripts/");
    
    // Get the output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let target_dir = Path::new(&out_dir).ancestors().nth(3).unwrap();
    let scripts_target_dir = target_dir.join("scripts");
    
    // Create the scripts directory in target
    if !scripts_target_dir.exists() {
        fs::create_dir_all(&scripts_target_dir).unwrap();
    }
    
    // Copy Julia scripts to target directory
    let scripts_source_dir = Path::new("scripts");
    if scripts_source_dir.exists() {
        copy_dir_recursive(scripts_source_dir, &scripts_target_dir).unwrap();
        println!("Copied Julia scripts to: {:?}", scripts_target_dir);
    }
    
    // Also copy to the deps directory where the process service creates them
    let deps_dir = target_dir.join("deps").join("scripts");
    if !deps_dir.exists() {
        fs::create_dir_all(&deps_dir).unwrap();
    }
    
    if scripts_source_dir.exists() {
        copy_dir_recursive(scripts_source_dir, &deps_dir).unwrap();
        println!("Copied Julia scripts to deps: {:?}", deps_dir);
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
            println!("Copied: {:?} -> {:?}", src_path, dst_path);
        }
    }
    
    Ok(())
}
