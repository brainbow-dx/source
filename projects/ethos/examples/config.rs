use std::collections::HashMap;
use std::process::ExitCode;

use eyre::Result;

use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug)]
#[derive(Serialize, Deserialize)]
struct Config {
    workspace: HashMap<String, Workspace>,
}

#[derive(Default, Debug)]
#[derive(Serialize, Deserialize)]
struct Workspace {
    key: Option<String>,
    some_key: Option<String>,
    project: HashMap<String, Project>,
}

#[derive(Default, Debug)]
#[derive(Serialize, Deserialize)]
struct Project {
    key: Option<String>,
    some_key: Option<String>,
}

//---
pub fn main() -> Result<ExitCode> {
    let source = include_str!("./config.ethos");
    let config = hcl::from_str::<Config>(source)?;

    println!("Parsed Value: {:#?}", config);

    Ok(ExitCode::SUCCESS)
}
