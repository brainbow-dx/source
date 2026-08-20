//! `.anvil.toml` is an optional, project-directory config pointing this app's `sqld`/Ollama clients
//! at specific addresses. `anvil init` writes it (see [`run_init`]); normal startup (`main`)
//! reads it, before `AppState::new` resolves its own `sqld_url` or anything spawns a JS command
//! that reads `ANVIL_OLLAMA_URL`. See that function's own doc comment for the load-order
//! contract this depends on.
//!
//! Deliberately only ever looked for in the current directory, never walked up toward a repo
//! root. This is meant to be an explicit, per-project override a person put there on purpose,
//! not something that silently starts applying because some unrelated parent directory happens
//! to have one.

use std::io::Write as _;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;

pub const CONFIG_FILE_NAME: &str = ".anvil.toml";

const DEFAULT_SQLD_URL: &str = "http://localhost:5100";
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

const PROBE_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AnvilConfig {
    pub sqld: Option<ServiceConfig>,
    pub ollama: Option<ServiceConfig>,
    pub window: Option<WindowConfig>,
    pub welcome: Option<WelcomeConfig>,
    /// Directories (relative to this project's own directory, same convention as everything else
    /// here) of `.js`/`.css` files mounted into every browser tab's webview at startup. See
    /// `extensions::load_extensions` and `spec/.agents/proposals/
    /// webview-script-injection-mvp.md` — a dev-tool mechanism, not a real extension runtime.
    pub extensions: Option<Vec<String>>,
}

/// Lets a project override the new-user welcome overview's one-line tagline and the small usage
/// note shown under the command palette (see `main.rs`'s own `WELCOME_TAGLINE`/`DEFAULT_FOOTER`
/// doc comments for exactly where each renders). `None`/absent fields keep this app's own
/// built-in default text, same as every other optional config here.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct WelcomeConfig {
    pub tagline: Option<String>,
    pub footer: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WindowConfig {
    /// See `Args::always_on_top`'s own doc comment. A CLI `--always-on-top` wins over this if
    /// both are given.
    #[serde(default)]
    pub always_on_top: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub url: String,
}

impl AnvilConfig {
    pub fn load_from_cwd() -> Option<Self> {
        let contents = std::fs::read_to_string(CONFIG_FILE_NAME).ok()?;
        match toml::from_str(&contents) {
            Ok(config) => Some(config),
            Err(error) => {
                tracing::warn!("Failed to parse {CONFIG_FILE_NAME}, ignoring it: {error}");
                None
            }
        }
    }

    fn save_to_cwd(&self) -> std::io::Result<()> {
        let contents = toml::to_string_pretty(self).expect("AnvilConfig always serializes");
        let mut file = std::fs::File::create(CONFIG_FILE_NAME)?;
        file.write_all(contents.as_bytes())
    }
}

/// `anvil init`'s whole job: get `sqld`/Ollama reachable, then record where at in
/// `.anvil.toml` so every later `anvil` launch in this directory skips straight to those
/// addresses (no probing, no `docker compose`) via [`AnvilConfig::load_from_cwd`]. Per the user
/// directly: never assumes either service needs starting at all. An explicit `--sqld-url`/
/// `--ollama-url` is trusted outright, and otherwise the existing default address is probed
/// first; `docker compose` is only ever reached for once nothing answers there.
pub fn run_init(sqld_url_override: Option<String>, ollama_url_override: Option<String>) {
    if Path::new(CONFIG_FILE_NAME).exists() {
        println!("{CONFIG_FILE_NAME} already exists here — leaving it alone.");
        println!("Delete it and re-run `anvil init` if you want to reconfigure.");
        return;
    }

    let sqld_url = resolve_service("sqld", "sqld-url", sqld_url_override, DEFAULT_SQLD_URL, "db0");
    let ollama_url = resolve_service("Ollama", "ollama-url", ollama_url_override, DEFAULT_OLLAMA_URL, "ollama");

    let config =
        AnvilConfig { sqld: Some(ServiceConfig { url: sqld_url }), ollama: Some(ServiceConfig { url: ollama_url }), window: None, welcome: None, extensions: None };

    match config.save_to_cwd() {
        Ok(()) => println!("Wrote {CONFIG_FILE_NAME} — future `anvil` launches here will use these addresses directly."),
        Err(error) => eprintln!("Failed to write {CONFIG_FILE_NAME}: {error}"),
    }
}

/// Resolves one service's address. `override_url` (an explicit `--sqld-url`/`--ollama-url`)
/// always wins and is trusted without probing. The whole point of passing one is pointing at
/// something this machine can't necessarily reach *yet* (a teammate's host, say). Otherwise
/// probes `default_url`; only if nothing answers there does this reach for `docker compose` (see
/// `compose_file_path`) to bring up `compose_service`, then probes again. Falls back to
/// `default_url` regardless of how that second probe goes. `anvil` itself already degrades
/// gracefully to running without persistence/without the Ollama fallback if the address it's
/// given doesn't actually work, same as it always has.
fn resolve_service(name: &str, flag_name: &str, override_url: Option<String>, default_url: &str, compose_service: &str) -> String {
    if let Some(url) = override_url {
        println!("{name}: using {url} (given explicitly, not probed).");
        return url;
    }

    if probe(default_url) {
        println!("{name}: already running at {default_url}.");
        return default_url.to_string();
    }

    println!("{name}: nothing listening at {default_url} yet.");

    match compose_file_path() {
        Some(compose_file) => {
            println!("{name}: starting via `docker compose` ({})...", compose_file.display());
            let status = Command::new("docker").args(["compose", "-f"]).arg(&compose_file).args(["up", "-d", compose_service]).status();

            match status {
                Ok(status) if status.success() => {
                    // Both `sqld` and Ollama take a moment to actually start accepting
                    // connections after the container itself reports running.
                    for _ in 0..10 {
                        if probe(default_url) {
                            println!("{name}: up at {default_url}.");
                            return default_url.to_string();
                        }
                        std::thread::sleep(Duration::from_millis(500));
                    }
                    eprintln!("{name}: container started but never answered at {default_url} — check `docker compose logs {compose_service}`.");
                }
                Ok(status) => eprintln!("{name}: `docker compose up` exited with {status}"),
                Err(error) => eprintln!("{name}: failed to run `docker compose`: {error}"),
            }
        }
        None => {
            eprintln!(
                "{name}: not found locally, and this binary isn't running from inside the escher \
                 checkout (no compose.yaml alongside it), so it can't start one automatically. \
                 Pass --{flag_name} to point at one instead, or start it yourself."
            );
        }
    }

    default_url.to_string()
}

/// A plain TCP connect, not an HTTP request. Good enough to answer "is anything listening
/// here," which is all every call site actually needs, without this module needing an HTTP
/// client dependency of its own.
///
/// Tries every address `to_socket_addrs` resolves `host_port` to, not just the first. This was
/// confirmed live as a real bug otherwise: `"localhost"` commonly resolves to both `::1` and `127.0.0.1`,
/// in unspecified order, and a service bound to only one of those (Ollama, in the case that
/// surfaced this) reads as "nothing listening" if the other one happens to sort first, even
/// though it's actually up.
fn probe(url: &str) -> bool {
    let Some(without_scheme) = url.split("://").nth(1) else { return false };
    let host_port = without_scheme.split('/').next().unwrap_or(without_scheme);
    let Ok(addrs) = host_port.to_socket_addrs() else { return false };
    addrs.into_iter().any(|addr| TcpStream::connect_timeout(&addr, PROBE_TIMEOUT).is_ok())
}

/// `apps/anvil`'s own `CARGO_MANIFEST_DIR` is `<repo>/projects/escher/apps/anvil`. Two levels up
/// is `projects/escher`, where `compose.yaml` actually lives. `None` when that file isn't there
/// (a shipped binary with no source checkout alongside it, say). `resolve_service` treats that
/// as "nothing to try," not an error worth failing `init` over.
fn compose_file_path() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../compose.yaml");
    path.exists().then_some(path)
}

/// Ensures `compose_service` is reachable at `default_url`, starting it via `docker compose` if
/// nothing's listening yet. Same probe-then-start logic `resolve_service` uses for `sqld`/
/// Ollama during `anvil init`, reused here for a service `anvil` itself wants running for as
/// long as it's running, not just something a user opts into via `init`. Per the user directly:
/// the escher dev server (`escher.brainbow.localhost`, via Caddy → `127.0.0.1:3615`) should
/// always be reachable while Anvil is running, so it's started proactively at every launch
/// rather than waiting for something to ask for it first. Meant to be called from a background
/// thread (`main`'s own call site does). This blocks on real process/network I/O throughout,
/// same as `resolve_service`.
pub(crate) fn ensure_docker_service_running(name: &str, default_url: &str, compose_service: &str) {
    if probe(default_url) {
        tracing::info!("{name}: already running at {default_url}.");
        return;
    }

    let Some(compose_file) = compose_file_path() else {
        tracing::warn!("{name}: nothing listening at {default_url}, and no compose.yaml found alongside this checkout to start it from.");
        return;
    };

    tracing::info!("{name}: nothing listening at {default_url} yet — starting via `docker compose`...");
    let status = Command::new("docker").args(["compose", "-f"]).arg(&compose_file).args(["up", "-d", compose_service]).status();

    match status {
        Ok(status) if status.success() => {
            for _ in 0..10 {
                if probe(default_url) {
                    tracing::info!("{name}: up at {default_url}.");
                    return;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            tracing::warn!("{name}: container started but never answered at {default_url} — check `docker compose logs {compose_service}`.");
        }
        Ok(status) => tracing::warn!("{name}: `docker compose up` exited with {status}"),
        Err(error) => tracing::warn!("{name}: failed to run `docker compose`: {error}"),
    }
}
