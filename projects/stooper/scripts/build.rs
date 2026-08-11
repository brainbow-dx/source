#![allow(unused_imports)]

use std::process::ExitCode;
use std::path::Path;

use color_eyre::Result;

//--
pub fn main() -> Result<ExitCode> {
    prost_build::compile_protos(&[
        "spec/protocol/setting.proto",
        "spec/protocol/user.proto",
    ], &[
        "spec/protocol",
    ])?;
    
    println!("cargo:rerun-if-changed=spec/protocol");
    println!("cargo:rerun-if-changed=src");
    
    Ok(ExitCode::SUCCESS)
}