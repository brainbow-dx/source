use std::fs::File;

use deno_runtime::deno_io::Stdio as DenoStdio;
use deno_runtime::deno_io::StdioPipe as DenoStdioPipe;

pub struct RuntimeStdio {
    stdin: DenoStdioPipe,
    stdout: DenoStdioPipe,
    stderr: DenoStdioPipe,
}

impl RuntimeStdio {
    /// `stdin` is always a fresh, empty scratch file — never the real process's stdin — so an
    /// embedded/spawned script can never block this process waiting on input meant for it.
    /// `stdout`/`stderr` inherit the real process's own handles unless a caller passes its own
    /// `File` to capture into instead (`deno_io::StdioPipe::inherit()` replaces what used to
    /// require cloning the removed `deno_io::STDOUT_HANDLE`/`STDERR_HANDLE` statics).
    pub fn try_new(stdout: Option<File>, stderr: Option<File>) -> Result<Self, std::io::Error> {
        Ok(RuntimeStdio {
            stdin: DenoStdioPipe::file(tempfile::tempfile()?),
            stdout: match stdout {
                Some(file) => DenoStdioPipe::file(file),
                None => DenoStdioPipe::inherit(),
            },
            stderr: match stderr {
                Some(file) => DenoStdioPipe::file(file),
                None => DenoStdioPipe::inherit(),
            },
        })
    }
}

impl RuntimeStdio {
    /// Turn a `RuntimeStdio` into a `deno_runtime::io::Stdio`, by cloning the inner pipes.
    pub fn try_clone_into(&self) -> Result<DenoStdio, std::io::Error> {
        Ok(DenoStdio {
            stdin: self.stdin.clone(),
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
        })
    }
}
