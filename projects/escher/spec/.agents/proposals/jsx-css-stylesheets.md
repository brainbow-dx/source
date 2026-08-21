# Regular stylesheets for JSX-authored commands (proposed, not started)

Flagged in PR review on `commands/shape.tsx`: it styles itself with `@escher/core/style`'s builder functions (`BackgroundColor`, `FlexDirection`, `Gap`, `Padding`, `px`, ...) — the same typed builders `escher-core`'s Rust side uses to construct a `StyleSheet`. The reviewer's direction: those builders are for the Rust side; JSX authored on the TypeScript side should style itself with regular stylesheets instead (plain CSS, or a CSS-in-JS convention — not yet decided which), not by importing and calling Rust-mirroring constructor functions from TS.

## Why this isn't a quick fix

There's no regular-stylesheet mechanism in `@escher/jsx` to switch to yet. `packages/jsx/src/mod.ts`'s `buildNode` only ever accepts a `Style | Style[]` (the exact same `@escher/core/style` type), and the package's own canonical example (`packages/jsx/examples/page.tsx`) uses the identical builder-function pattern `shape.tsx` does — this isn't a shape.tsx-specific mistake, it's the only styling path that exists today. Building a real one means deciding:

- Plain `.css` files imported alongside a `.tsx`, parsed and compiled down to the same `ScaffoldNode`-embedded `Style` list `@escher/jsx` already produces — needs a real (or vendored) CSS parser and a mapping from CSS properties to `escher-core`'s `Property` enum, which won't cover 1:1 (this workspace's `Style` set is deliberately smaller/typed, not full CSS).
- Or a CSS-in-JS convention (tagged template, object literal) that still compiles to the same target — less parsing work, but invents a syntax rather than using one everyone already knows.
- Either way, `packages/jsx/src/mod.ts`'s `buildNode` needs a second input path alongside (or instead of) `Style | Style[]`, and every existing JSX consumer (`examples/page.tsx`, `commands/shape.tsx`) needs to move onto whichever shape wins.

## Status

Proposed only. `commands/shape.tsx` still uses the builder-function pattern for now — matches the package's own current (and only) capability, not yet the direction the reviewer wants. Not blocking a build; worth resolving before more JSX-authored commands accumulate the same pattern.
