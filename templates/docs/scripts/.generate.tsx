#!/usr/bin/env deno
// Bootstraps a live-reloading mdbook doc site into a target project: copies this template's
// docs/ scaffold, then adds (or creates) a `docs:` service in the target's `compose.yaml`.
//
// Deliberately depends on nothing but `@std/*` (both real entries in the workspace root
// `deno.jsonc`'s import map) — `package`/`app`/`service`'s own generate scripts import
// `@ethos/dev/shell`/`@ethos/flutter`, neither of which resolves from that same import map today,
// so those scripts can't actually run as-is. This one can, standalone or composed with any of
// those templates, which is the actual point of a small, focused template.
import { join, resolve, basename } from "@std/path";
import { exists } from "@std/fs";
import { parseArgs } from "@std/cli/parse-args";

const args = parseArgs(Deno.args, {
    string: ["name"],
    boolean: ["verbose"],
    default: { port: 8096, verbose: false },
});

const tpldir = resolve(import.meta.dirname!, "..");
const OUT_DIR = resolve(Deno.cwd(), args._[0]?.toString() ?? ".");
const DOCS_DIR = join(OUT_DIR, "docs");
// Book content lives under `spec/docs/`, not `docs/` itself — `docs/` only ever holds the
// Dockerfile/book.toml, matching where every other project's own book content already lives.
const SPEC_DOCS_DIR = join(OUT_DIR, "spec", "docs");
const COMPOSE_PATH = join(OUT_DIR, "compose.yaml");
const port = Number(args.port);

const name = args.name ?? basename(OUT_DIR);
if (!args.name) {
    console.info(`Missing --name flag; using directory name '${name}' ..`);
}

console.log(`docs: generating docs/ @ '${DOCS_DIR}', spec/docs/ @ '${SPEC_DOCS_DIR}'`);

await Deno.mkdir(DOCS_DIR, { recursive: true });
await Deno.mkdir(SPEC_DOCS_DIR, { recursive: true });

// Fixed scaffold files: Dockerfile stays docs-tooling-only; the actual book content (SUMMARY.md,
// content pages) goes to spec/docs/ instead.
await Deno.copyFile(join(tpldir, "Dockerfile"), join(DOCS_DIR, "Dockerfile"));
if (args.verbose) console.log(`> copy docs/Dockerfile`);

for (const relPath of ["SUMMARY.md", "Introduction.md"]) {
    const from = join(tpldir, "spec-docs", relPath);
    const to = join(SPEC_DOCS_DIR, relPath);
    await Deno.copyFile(from, to);
    if (args.verbose) console.log(`> copy spec/docs/${relPath}`);
}

const bookToml = (await Deno.readTextFile(join(tpldir, "book.toml")))
    .replace("__PROJECT_NAME__", name);
await Deno.writeTextFile(join(DOCS_DIR, "book.toml"), bookToml);
if (args.verbose) console.log(`> copy docs/book.toml (title: ${name})`);

// `compose.yaml`: add a `docs` service if the project already has a compose file (matching
// whatever indentation/shape it already uses isn't attempted — a plain 2-space block appended
// right after `services:` matches every real Brainbow project's own compose.yaml today), or
// write a fresh minimal one if it doesn't. Idempotent: does nothing if a `docs:` service already
// exists, so re-running this template on an already-composed project is a safe no-op.
const DOCS_SERVICE = `  # \`docker compose --profile docs up\` alone serves the static book built into the image at
  # :${port} (the Dockerfile's own release default). \`docker compose --profile docs watch\`
  # instead overrides the command to \`mdbook serve\`, which live-reloads: \`develop.watch\` syncs
  # local edits into the container, mdbook's own file watcher picks those up and rebuilds.
  docs:
    build:
      context: .
      dockerfile: docs/Dockerfile
    profiles: [docs]
    ports:
      - ${port}:3000
    command: ["mdbook", "serve", "--hostname", "0.0.0.0", "--port", "3000"]
    develop:
      watch:
        - path: ./spec/docs
          action: sync
          target: /spec/docs
`;

if (await exists(COMPOSE_PATH)) {
    const contents = await Deno.readTextFile(COMPOSE_PATH);
    if (/^\s*docs:\s*$/m.test(contents)) {
        console.log(`compose.yaml already has a 'docs' service; leaving it alone.`);
    } else {
        const updated = contents.replace(/^services:\s*\n/m, (match) => match + DOCS_SERVICE);
        if (updated === contents) {
            console.warn(`Couldn't find a 'services:' line in compose.yaml — not touching it. Add the docs service manually; see templates/docs/README.md.`);
        } else {
            await Deno.writeTextFile(COMPOSE_PATH, updated);
            console.log(`Added 'docs' service to existing compose.yaml.`);
        }
    }
} else {
    await Deno.writeTextFile(COMPOSE_PATH, `services:\n${DOCS_SERVICE}`);
    console.log(`Created compose.yaml with a 'docs' service.`);
}

console.log(`Fin. Serve it: cd ${OUT_DIR} && docker compose --profile docs watch`);
