//! The real top-level `escher` entrypoint — `escher anvil` instead of the raw `cargo run -p
//! escher-anvil --bin escher-anvil` every session has used so far. See
//! `spec/.agents/proposals/terminal-drawing-and-embedding.md`'s item 4 for the design history:
//! a bare flag on one binary was rejected in favor of a real `clap` subcommand tree, the same
//! shape `cargo` itself uses, specifically so a later `--tui` mode lands as a flag *on* `anvil`
//! rather than another top-level mode. `ROADMAP.md` (M6) tracks this as a real blocker for that
//! work — this is the stub that unblocks it, not the `--tui` feature itself.
//!
//! Deliberately a thin dispatcher, not a merge of `apps/anvil`'s own binary into this crate:
//! `anvil` stays exactly what it is today (its own crate, its own heavy Bevy/AppKit/webview
//! dependency graph, no `lib` target to link against), so this just execs the built binary
//! instead — see [`resolve_subcommand_binary`] for exactly how it finds one. Per the user
//! directly: this chain should exec real, already-built release binaries when they're available,
//! not shell out to `cargo run` — that only ever worked from inside this monorepo checkout with
//! `cargo` on `PATH`, which is exactly wrong for testing an actually-built release install (e.g.
//! `~/.bin/escher`). `cargo run` is now the last-resort fallback, used only from inside a dev
//! checkout with neither a sibling nor a `PATH` binary installed, and it clearly logs that it's
//! doing so rather than silently taking the slow path.

use std::path::PathBuf;
use std::process::Command;
use std::process::ExitCode;

use clap::Parser;

/// The `escher` CLI — one subcommand per app/tool that lives in this workspace, `cargo`-style.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Scaffolds a new Escher project in the current directory — the smallest real thing that
    /// works as a project `escher anvil` can then open: a `Cargo.toml` (with a
    /// `[package.metadata.anvil]` table, so a mounted Anvil can read this project's own
    /// title/tagline rather than always showing its own — see `ROADMAP.md`'s "Anvil as a
    /// mountable exoskeleton" note), the smallest possible `src/main.rs`, a `pages/` directory
    /// with one real example `Scaffold`/HTML page, and a `spec/`/`spec/.agents/` doc skeleton
    /// mirroring this very workspace's own convention.
    Init {
        /// Where to scaffold the project — defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Launches Anvil (the terminal + Bevy + native webview app). Everything after `anvil` is
    /// forwarded as-is — this subcommand doesn't parse Anvil's own flags itself, so adding a new
    /// one to `apps/anvil` (like the planned `--tui`) never requires touching this crate too.
    Anvil {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> ExitCode {
    let args = Args::parse();

    match args.command {
        Commands::Init { path } => match escher_init::run_init(&path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("escher init failed: {error}");
                ExitCode::FAILURE
            }
        },
        Commands::Anvil { args } => exec_subcommand("escher-anvil", &args),
    }
}

/// Runs `binary_name` (found via [`resolve_subcommand_binary`]) with `args`, forwarding this
/// process's stdio and exit code unchanged — so e.g. `escher anvil`'s own exit status still means
/// what it should to a script or CI step calling it.
fn exec_subcommand(binary_name: &str, args: &[String]) -> ExitCode {
    let (program, base_args) = resolve_subcommand_binary(binary_name);

    let status = Command::new(program).args(base_args).args(args).status();

    match status {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("failed to launch {binary_name}: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Finds `binary_name` the same way `cargo`/`git` find their own external subcommands, via
/// `escher_os::process::find_sibling_or_path` (shared with `apps/anvil`'s own `ethos-cli`
/// lookup, since the same sibling-of-exe/`PATH` resolution logic applies to both):
///
/// 1. A binary named `binary_name` sitting right next to this `escher` executable itself — the
///    real, self-contained release-install case (`~/.bin/escher` shipped alongside
///    `~/.bin/escher-anvil`), and the one this exists to make actually work.
/// 2. `binary_name` anywhere on `PATH` (a `cargo install`-style setup, one directory holding
///    every `escher-*` binary without also holding this exact `escher` executable).
/// 3. Last resort, dev-only: `cargo run --release -p <crate> --bin <binary_name>`, `<crate>`
///    being `binary_name` itself (matches this workspace's own package-naming convention, e.g.
///    `escher-anvil`'s crate and binary share a name) — only reachable from inside a checkout
///    that actually has this binary's crate and `cargo` on `PATH`, logged clearly so it's obvious
///    why a first launch is slow.
///
/// Returns `(program, leading_args)` — `leading_args` is empty for cases 1/2 (the resolved path
/// *is* the program to run), non-empty for case 3 (`cargo`'s own subcommand/flags come first).
fn resolve_subcommand_binary(binary_name: &str) -> (PathBuf, Vec<String>) {
    if let Some(found) = escher_os::process::find_sibling_or_path(binary_name) {
        return (found, Vec::new());
    }

    eprintln!("{binary_name}: no release binary found next to `escher` or on PATH — falling back to `cargo run` (slow; build a release binary and put it alongside `escher` to skip this).");
    ("cargo".into(), ["run", "--quiet", "--release", "-p", binary_name, "--bin", binary_name, "--"].into_iter().map(String::from).collect())
}

/// Kept in this file rather than a separate module — small enough, and `apps/cli` has no other
/// modules yet to make a `mod` split worth it. See [`Commands::Init`] for the user-facing contract
/// this implements.
mod escher_init {
    use std::fs;
    use std::io;
    use std::path::Path;

    pub fn run_init(path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)?;

        let project_name = path
            .canonicalize()
            .ok()
            .and_then(|absolute| absolute.file_name().map(|name| name.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "escher-project".to_string());

        write_new(&path.join("Cargo.toml"), &cargo_toml(&project_name))?;

        fs::create_dir_all(path.join("src"))?;
        write_new(&path.join("src/main.rs"), MAIN_RS)?;

        fs::create_dir_all(path.join("pages"))?;
        write_new(&path.join("pages/index.html"), EXAMPLE_PAGE)?;

        fs::create_dir_all(path.join("spec/.agents/proposals"))?;
        write_new(&path.join("spec/README.md"), &spec_readme(&project_name))?;
        write_new(&path.join("spec/.agents/changelog.md"), CHANGELOG_STUB)?;
        write_new(&path.join("spec/.agents/handoff.md"), HANDOFF_STUB)?;

        println!("Scaffolded a new Escher project in {}", path.display());
        println!("Next: cd {} && escher anvil", path.display());
        Ok(())
    }

    /// Refuses to clobber anything already there — `escher init` in a directory that already
    /// has, say, a hand-written `Cargo.toml` should fail loudly, not silently overwrite it.
    fn write_new(path: &Path, contents: &str) -> io::Result<()> {
        if path.exists() {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} already exists", path.display())));
        }
        fs::write(path, contents)
    }

    fn cargo_toml(project_name: &str) -> String {
        format!(
            "[package]\n\
             name = \"{project_name}\"\n\
             version = \"0.1.0\"\n\
             edition = \"2024\"\n\
             \n\
             # Read by a mounted Anvil in place of its own default title/tagline — see this\n\
             # workspace's own `spec/ROADMAP.md`, \"Anvil as a mountable exoskeleton.\"\n\
             [package.metadata.anvil]\n\
             title = \"{project_name}\"\n\
             tagline = \"A new Escher project.\"\n\
             \n\
             [dependencies]\n"
        )
    }

    fn spec_readme(project_name: &str) -> String {
        format!(
            "# {project_name}\n\
             \n\
             An Escher project. `pages/index.html` is a real `<escher-scaffold>` page — open it\n\
             with `escher anvil` (or any Escher-aware surface) to see it rendered.\n\
             \n\
             `spec/.agents/` mirrors this workspace's own convention: `changelog.md` (terse,\n\
             one-line-per-change, newest at the top) and `handoff.md` (overwritten each session,\n\
             not appended to — always describes *current* state for whoever picks this up next).\n"
        )
    }

    const MAIN_RS: &str = "\
fn main() {
    println!(\"This project's real content lives in pages/ and spec/ — open it with `escher anvil`.\");
}
";

    const EXAMPLE_PAGE: &str = "\
<!doctype html>
<html lang=\"en\">
<head>
<meta charset=\"utf-8\" />
<title>A new Escher project</title>
<script type=\"module\" src=\"/scaffold-element.js\"></script>
</head>
<body style=\"margin: 0; background: #000;\">
<escher-scaffold>
  <div style=\"display: flex; flex-direction: column; gap: 12px; padding: 24px; color: #e2e2e2; font-family: system-ui;\">
    <h1 style=\"margin: 0;\">Hello from a new Escher project</h1>
    <p style=\"margin: 0; opacity: 0.7;\">
      This is a real, editable <code>Scaffold</code> page — not a placeholder screenshot.
    </p>
    <div style=\"display: flex; gap: 8px;\">
      <button style=\"padding: 8px 14px;\">A button</button>
      <input placeholder=\"A text field\" style=\"padding: 8px 10px;\" />
    </div>
  </div>
</escher-scaffold>
</body>
</html>
";

    const CHANGELOG_STUB: &str = "# Changelog\n\nTerse, one-line-per-entry, newest at the top.\n";

    const HANDOFF_STUB: &str = "# Handoff\n\nOverwrite this each session with *current* state — don't append.\n\n(Nothing built yet.)\n";
}
