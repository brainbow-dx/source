#![allow(unused_imports)]

use std::path::Path;
use std::env;
use std::fs;

//---
pub fn main() {
    // // 1. Define the desired output path relative to the current crate
    // let dest_dir = Path::new(".output/pkg/web/public/wasm");
    
    // // 2. Define the source path (standard Cargo release target)
    // let profile = env::var("PROFILE").unwrap(); // "release" or "debug"
    // let crate_name = env::var("CARGO_PKG_NAME").unwrap().replace("-", "_"); // Get crate name
    
    // // Construct the path to the WASM file inside the target directory
    // let wasm_file_name = format!("{}.wasm", crate_name);
    // let src_path = Path::new(&env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| String::from("target")))
    //     .join("wasm32-unknown-unknown")
    //     .join(profile)
    //     .join(wasm_file_name);
    
    // // 3. Ensure the destination directory exists
    // fs::create_dir_all(&dest_dir)
    //     .expect("Failed to create destination directory.");
    
    // // 4. Copy the file
    // let dest_path = dest_dir.join(format!("{}.wasm", crate_name));
    // fs::copy(&src_path, &dest_path)
    //     .unwrap_or_else(|_| panic!("Failed to copy WASM file from {:?} to {:?}", src_path, dest_path));

    // // Optional: Tell Cargo to rerun this build script if the Wasm file changes
    // println!("cargo:rerun-if-changed={}", src_path.display());
}