# docs template

Bootstraps a live-reloading [mdBook](https://rust-lang.github.io/mdBook/) documentation site for a project — the smallest of the Brainbow templates, and meant to be composed *with* one of the others (`app`/`package`/`service`/`workspace`) rather than used alone: it never touches `src/`, `Cargo.toml`, or `deno.json`, only `docs/` and a `docs:` service in the project's `compose.yaml`.

## What it does

- Copies `Dockerfile`/`book.toml`/`SUMMARY.md`/`src/Introduction.md` into `<target>/docs/`.
- Adds (or creates) a `docs:` service in `<target>/compose.yaml` — a dev-mode container running `mdbook serve`, paired with a `develop.watch` block so `docker compose --profile docs watch` syncs local edits straight in; mdbook's own file watcher rebuilds + live-reloads from there, no container restart needed. Idempotent — running it again on a project that already has the service is a no-op, not a duplicate block.

## Usage

```sh
deno run -A scripts/.generate.tsx --name my-project /path/to/my-project
```

`--name` sets the book's title (defaults to the target directory's basename, same convention as the other templates). Serves at `:8096` by default — pass `--port` to change it if a project already has something bound there.
