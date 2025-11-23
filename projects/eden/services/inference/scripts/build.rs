use std::process::ExitCode;

use anyhow::Result;

//---
fn main() -> Result<ExitCode> {
    println!("cargo:rerun-if-changed=src");

    Ok(ExitCode::SUCCESS)
}
