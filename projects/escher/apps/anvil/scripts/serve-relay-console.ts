// Serves `apps/anvil/assets/relay-console` as plain static files — the Relay Console's real,
// permanent home (see its own `index.html` doc comment... it has none, it's plain HTML/CSS/JS,
// no build step at all, same reasoning `runtimes/web`'s own dev server has for keeping its own
// static legs simple). Deno's own `serveDir`, not a Rust/axum service — per the repo owner
// directly, a script-driven server like this should reach for what Deno already gives us, not
// stand up a whole Rust web framework for "serve one static directory."
//
// Invoked by `apps/anvil/src/main.rs`'s `spawn_relay_console_server` as a background child
// process (`deno run --allow-net --allow-read`), the same "spawn a subprocess, don't link a web
// framework into the terminal app itself" shape `process::run_deno_command` already uses for
// everything else this app shells out to.
import { serveDir } from "@std/http/file-server";

const port = Number(Deno.args[0] ?? 4002);
const root = new URL("../assets/relay-console", import.meta.url).pathname;

Deno.serve({ port, hostname: "127.0.0.1" }, (request) => serveDir(request, { fsRoot: root }));
