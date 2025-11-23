#![allow(unused)]
#![feature(allocator_api)]

extern crate alloc;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser as ArgParser;

use eyre::Result;

use derive_more::Deref;
use derive_more::DerefMut;
use derive_more::Display;

#[derive(ArgParser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "trace")]
    log_filter: String,

    #[arg(short, long)]
    inspect: bool,

    #[arg(default_value = "./examples/template.tsx")]
    entrypoint: PathBuf,
}

//---
/// Usage: ./scripts/instruments path/to/input/file
fn main() -> Result<ExitCode> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&args.log_filter)
        .with_level(true)
        .without_time()
        .with_target(true)
        .with_ansi(true)
        .init();

    // TODO

    Ok(ExitCode::SUCCESS)
}
