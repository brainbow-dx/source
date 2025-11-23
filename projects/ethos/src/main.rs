#![allow(unused)]

#[cfg(not(feature = "std"))]
compile_error!("Std is required for full binaries.");

//---
extern crate alloc;

pub mod error;
pub mod log;

use std::path::PathBuf;
use std::process::ExitCode;

use oops::DogHouse;

use clap::ArgAction;
use clap::Parser;
use clap::Subcommand;

use crate::error::EthosError;

//---
/// Hello??
#[derive(Parser, Debug)]
#[command(name = "MyApp", version = "1.0")]
#[command(about = "Does awesome things", long_about = None)]
struct Args {
    /// The minimum log level.
    #[arg(long, default_value = "trace")]
    log_level: String,

    /// The command to run.
    #[command(subcommand)]
    command: Option<Commands>,

    /// How loud should we be?
    #[arg(short, action = ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Generate {
        #[arg(long)]
        dry: Option<bool>,

        #[arg(short, long)]
        name: Option<String>,

        //---
        #[arg(default_value = ".")]
        template: String,

        #[arg(default_value = ".")]
        target_dir: PathBuf,
    },
}

fn main() -> Result<ExitCode, EthosError> {
    let args = Args::parse();

    crate::log::init(&args.log_level);

    #[cfg(all(feature = "debug", feature = "verbose"))]
    tracing::debug!("Got cli args:\n{args:#?}");

    if let Some(command) = args.command {
        match command {
            Commands::Generate {
                template,
                target_dir,
                dry,
                ..
            } => {
                tracing::debug!("Template Specifier: {0}", template);
                tracing::debug!("Target Directory: {0}", target_dir.display());

                if !target_dir.exists() {
                    // TODO: Do we need to create it first?
                    // std::fs::create_dir_all(entrypoint)?;
                }

                // TODO: Walk the template directory and collect file paths.

                if let Some(_) = dry {
                    tracing::debug!("Dry run ..");
                }

                // TODO: Iterate over found paths and copy contents.
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

//---
mod oops {
    use super::*;

    pub use ::oops::*;

    //---
    #[derive(Default, Clone, Copy)]
    pub struct DogHouse;

    impl DogHouse {
        pub fn new() -> Self {
            DogHouse
        }
    }

    impl DogHouse {
        pub fn sniff<E: core::error::Error>(&self, _alert: bool) -> fn(E) -> E {
            // Hand out a notice about the infraction.
            |error| {
                tracing::error!("{error}");
                error // return to caller ..
            }
        }
    }
}
