# Anvil `/draw` command: serve `escher-web`'s `draw` surface via `anvil://draw`

Status: proposal, not started. Scoped from a direct user question ("does anvil yet have a
self-contained command script that builds escher-web to `.output` and serves it from
`anvil://draw`?") — answer was "not yet, but the exact pattern already exists and is proven."

## What already exists (no new plumbing needed)

This session already built and proved the full mechanism this needs, via `commands/docs.js`:

- `ethos-deno`'s `run_module_command`/`apps/anvil`'s `process::run_js_command` support an
  `export_name` parameter, so a command script can export `onLoad` (run once at Anvil startup,
  distinct from `run`, which runs per invocation).
- Two host actions a script can post via the existing `globalThis.__ethosHostAction` mechanism:
  `{ type: "mountStaticDir", prefix, dir }` (mounts a real directory under `anvil://<prefix>/...`)
  and `{ type: "openUrl", url }` (opens an `anvil://` URL in a tab).
- `escher-webview`'s `CustomSchemeHandler` serves real static files with correct MIME typing
  (`SchemeResponse { mime, body }`), not one hardcoded page — confirmed working for a real
  multi-file static site (`docs.js`'s mdbook output: CSS/JS/images/search index).

`commands/docs.js` is the reference implementation: `onLoad` shells out to `mdbook build` (found
via `import.meta.url`, not an assumed cwd), posts `mountStaticDir` with `prefix: "docs"`, and `run`
posts `openUrl` for `anvil://docs/`.

## What's actually different for `/draw`

`escher-web`'s own build (`runtimes/web/scripts/build.tsx`, invoked via `deno task build` from
`runtimes/web`) runs `wasm-pack build --target web --out-dir .output/pkg/web` plus a `deno bundle`
over `assets/{index,404,draw,download/index}.html`, producing real static output at
`runtimes/web/.output/pkg/web/` — including `draw.html`, the real entry point `anvil://draw` should
open. Confirmed by reading the build script directly, not assumed.

## Implementation sketch

New `apps/anvil/commands/draw.js`, mirroring `docs.js`'s shape:

- `onLoad`: `Deno.Command` runs `deno task build` with `cwd` set to `runtimes/web` (resolved via
  `import.meta.url`, same pattern `docs.js` uses to avoid an assumed working directory — `docs.js`
  needed 3 levels up from `apps/anvil/commands/` to reach the escher project root; verify the
  correct relative depth to `runtimes/web` the same way rather than assuming it matches). Posts
  `{ type: "mountStaticDir", prefix: "draw", dir: "<resolved path>/.output/pkg/web" }`.
- `run`: posts `{ type: "openUrl", url: "anvil://draw/draw.html" }` — confirm `draw.html` is
  actually the right entry file (not `index.html`) by checking what `assets/draw.html` bundles to
  and whether it's meant to stand alone or load `index.html`'s shell first.

## Open questions for whoever picks this up

- `escher-web`'s build is a real `wasm-pack` + `deno bundle` pipeline, slower than `mdbook build` —
  worth checking actual build time before deciding whether `onLoad` (blocking startup) is still the
  right lifecycle, or whether this should be lazier (e.g. build only on first `/draw` invocation,
  or a `--watch` mode like the docs devserver already uses elsewhere in this repo).
- Confirm `draw.html`'s actual runtime requirements — does it expect query params, a specific
  scaffold/description payload passed in some way, or does it run standalone with no input? This
  wasn't checked; `docs.js`'s mdbook output has no such requirement, so this may not directly
  transfer.
- Whether `.output/pkg/web`'s `wasm-pack` output needs the `wasm32-unknown-unknown` target/toolchain
  present in the environment running Anvil, and what happens (real error surfaced, silent failure?)
  if it isn't.

## Estimate

Roughly 15-20 minutes of focused work once picked up: write the ~20-line script, run a real
`deno task build` once to confirm output paths/filenames match what's assumed above, and verify
`mountStaticDir`/`openUrl` actually serve it through Anvil's real `anvil://` scheme handler the same
way `docs.js` already does live.
