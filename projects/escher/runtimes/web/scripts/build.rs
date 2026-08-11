extern crate cbindgen;

use std::path::PathBuf;
use std::process::ExitCode;
use std::env;

use eyre::Result;

use cbindgen::Builder as CBuilder;

//---
const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn main() -> Result<ExitCode> {
    let cbindgen_output = PathBuf::from(CARGO_MANIFEST_DIR)
        .join(".output/includes/escher_web.h");
    
    CBuilder::new()
        .with_crate(CARGO_MANIFEST_DIR)
        .generate()?.write_to_file(cbindgen_output);
    
    Ok(ExitCode::SUCCESS)
}