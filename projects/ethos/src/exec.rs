use std::process::Command;

const UNKNOWN_EXIT_CODE: i32 = -1;

pub struct Exec {
    _command: Command,
}

impl Exec {
    pub fn new<S: AsRef<std::ffi::OsStr>>(program: S) -> Self {
        Exec {
            _command: Command::new(program)
        }
    }

    /// TODO: Construct an Exec and use it here.
    pub fn run<S: AsRef<std::ffi::OsStr>, I: IntoIterator<Item = S>>(program: S, args: I) -> Result<ExecOutput, ExecError> {
        let args: Vec<S> = args.into_iter().collect();

        #[cfg(feature = "dev")]
        tracing::info!("Executing: {:?} {:}", program.as_ref(), args.iter().map(|arg| arg.as_ref().to_string_lossy()).collect::<Vec<_>>().join(" "));

        let mut command = Command::new(program);

        let result = command.args(args).output()
            .map_err(ExecError::RunFailed)?;

        let code = result.status.code()
            .unwrap_or(UNKNOWN_EXIT_CODE);

        if result.status.success() {
            let stdout = String::from_utf8_lossy(&result.stdout);
            Ok(ExecOutput(code, stdout.to_string()))
        } else {
            let stdout = String::from_utf8_lossy(&result.stderr);
            Err(ExecError::CommandFailed(code, stdout.to_string()))
        }
    }
}

pub struct ExecOutput(pub i32, pub std::string::String);

#[derive(oops::Error)]
pub enum ExecError {
    #[msg("run failed: {0}")]
    RunFailed(std::io::Error),

    #[msg("command failed with exit #{0}: {1}")]
    CommandFailed(i32, std::string::String),

    #[msg("unknown error, {0}")]
    Unknown(std::string::String),
}
