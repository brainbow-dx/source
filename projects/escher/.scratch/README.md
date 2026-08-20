Pre-refactor content kept for reference, not wired into any crate's build. Parallel to
`ethos/scratch/`.

- **`mcp/`, `relay/`** — moved from `runtimes/terminal/examples/` (dated Dec 2025, predating
  the terminal runtime's real example set). A generic JRPC/MCP HTTP server and a WebRTC
  signaling relay; neither references anything in `escher-terminal` and both sit outside
  the domain of a terminal-rendering crate. No `mcp`/`relay` crate exists yet to give them
  a real home.
- **`cwrap/`** — moved from `runtimes/terminal/examples/cwrap/repr-c.tsx`. C struct-layout
  reflection decorators (`$struct`, `$hidden`, `$locked`, `$method`), self-flagged with its
  own `// TODO: Move all of the following out to Ethos/Scribe and/or Cwrap.` comment. No
  `cwrap` crate exists yet either.
