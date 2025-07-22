mod service;

//---
use std::process::ExitCode;

use anyhow::Result;

use crate::service::RegistryService;

pub fn main() -> Result<ExitCode> {
    let service = RegistryService::new();
    Ok(ExitCode::SUCCESS)
}
