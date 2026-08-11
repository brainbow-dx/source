#![allow(unused_imports)]
extern crate napi_build;

use std::process::ExitCode;
use std::path::Path;

use color_eyre::Result;

//--
pub fn main() -> Result<ExitCode> {
    prost_build::compile_protos(&[
        "spec/protocol/user.proto",
        "spec/protocol/setting.proto",
        "spec/protocol/doodle.proto",
        "spec/protocol/resource.proto",
    ], &[
        "spec/proto",
    ])?;
    
    // TODO: Generate at-proto code from lexicons?
    
    println!("cargo:rerun-if-changed=spec/protocol");
    println!("cargo:rerun-if-changed=src");
    
    napi_build::setup();
    
    Ok(ExitCode::SUCCESS)
}