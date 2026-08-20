# Proposal: markdown+YAML-frontmatter styleguides, with CSS/web-standards parity as the long-term target

Status: **v1 minimal slice implemented (2026-08-15, human-insisted, time-boxed)** — smallest subset that unblocks a shared, consistent look across Anvil's terminal and native AppKit chrome, not the full design below. See "What actually shipped" at the bottom. The open question about component dependencies (§3/bottom) is still genuinely open — v1 doesn't touch it at all.

## The ask

Each project can define one or more styleguides — simple markdown documents with YAML frontmatter describing style values — which runtimes and codegen tools apply at compile time or runtime to inform Scaffold styling decisions. Stated explicit long-term target: **parity with CSS and other web standards across the board**, not an Escher-invented format. The human wants this in place before hand-styling any of the windows currently being built (the AppKit toolbar/tab-strip from this session's other work), since styling ad hoc now would be wasted effort once a real styleguide concept exists.

**Concrete first consumer, confirmed 2026-08-15**: the moment this exists, apply it across `apps/anvil` as a whole — the human's stated goal is one consistent-but-simple styleguide driving all four of Anvil's surfaces at once: the terminal UI, the AppKit toolbar/tab-strip windows, the Bevy scene, and webview content. This is the concrete proof-of-value case to build against once the parser/lookup API lands — don't design the schema in the abstract only to find it doesn't actually work cleanly across all four surface types when Anvil tries to consume it.

## Current state (verified via search, 2026-08-15)

**Not implemented at all.** No `serde_yaml`/`gray_matter`-shaped frontmatter-parsing dependency exists anywhere in the workspace (`grep`-confirmed across every `Cargo.toml`). `spec/design/styleguide/brand.md` exists and matches the target *shape* (YAML frontmatter + markdown body, `type: styleguide`), but nothing in the Rust codebase reads it — it's an example of the intended format, not a wired-up feature. `escher-chalk`'s own doc comment (added this session) already anticipates this: "the counterpart to a project's own styleguide... eventually consumed here and by runtimes/compilers to apply them; not implemented yet."

## Design requirements, refined with the human

1. **Variables work like CSS custom properties.** A `variables:` block in frontmatter defines named values; anything else in the document (and eventually `.with_style()` calls in composition code) references them by name rather than embedding literals, so changing one variable propagates everywhere it's used. Requires a resolution pass: collect all variable definitions, expand references (including variables referencing other variables), detect cycles — the same problem the browser solves for `var(--name)`.
2. **Tokens should track the W3C Design Tokens Community Group format**, not an ad hoc flat key-value shape, given the explicit CSS/web-standards-parity goal. Concretely: each token has a typed `$value`/`$type` (`color`, `dimension`, `fontFamily`, `fontWeight`, `duration`, `cubicBezier`, `number`, etc.) and can alias another token via reference syntax (`{token.path}`), rather than being an untyped raw value. This is real, existing prior art (Figma, Style Dictionary, and other design tools already interoperate via this format) — worth tracking it directly rather than inventing a incompatible Escher-specific shape.
3. **Component dependencies** — styleguides can express dependencies at the component level. **Open question, unresolved as of 2026-08-15** (see bottom).

## Proposed v1 scope (parser + runtime lookup only)

- A parser: split a markdown file's YAML frontmatter from its body (`---`-delimited), parse the YAML into a `Styleguide` struct.
- `Styleguide` shape: known top-level categories (`variables`, tokens organized by category) plus an open/extensible catch-all so the schema isn't locked to whatever categories exist on day one.
- A runtime-only lookup API (e.g. `styleguide.color("primary")`, `styleguide.token("typography.heading-weight")`) that composition code calls when building a `StyleSheet`.
- Variable/token reference resolution (§1/§2 above) as a load-time pass, not resolved lazily per-lookup.

## Explicitly deferred to a follow-up (not v1)

- **Compile-time/codegen**: generating Rust constants from a styleguide (so composition code gets compile-time-checked references instead of stringly-typed lookups) is a genuinely separate tool — likely a build-time pass or proc-macro. Building this alongside the runtime parser risks neither being solid; do the runtime path first, prove it out against real usage, then codegen on top.
- CSS cascade/specificity/selector matching — the human's "CSS parity" goal is about the *token/variable* model, not necessarily about reimplementing selectors; revisit only if a concrete need for scoped/conditional style resolution (media-query-equivalent, dark-mode-equivalent) shows up.

## Open question — needs the human's answer before the schema is final

**"Component dependencies"** was named as a concern but not resolved. Two plausible readings, which produce materially different schemas:

- **(a)** A styleguide declares `extends`/imports of other styleguide files — CSS-`@import`-like. This would also resolve the *other* open question (how do multiple styleguides for one project combine — explicit declared order instead of an implicit last-loaded-wins convention).
- **(b)** A styleguide declares which tokens a given *component* requires — a contract a linter/compiler tool could check components against (e.g. "any `Button` composition needs `color.primary` and `spacing.button-padding` defined").
- Both are plausible and not mutually exclusive, but which one (or both) ships in v1 changes the frontmatter schema directly. Do not start implementation until this is answered.

## What actually shipped (v1, 2026-08-15)

Time-boxed to "smallest subset that puts a solid foundation in place and gets the browser chrome
looking consistent with the TUI" — a much smaller slice than the full design above, on the human's
explicit instruction ("I don't have much time"). Concretely:

- **`packages/styleguide`** (`escher-styleguide`): parses a `---`-delimited YAML frontmatter block
  into a flat `colors: HashMap<String, (u8,u8,u8)>` (hex strings only) + `dimensions:
  HashMap<String, f64>`. No variable/token aliasing (`{token.path}` references — §1 above), no W3C
  `$type`/`$value` tagging (§2 above), no component-dependency declarations (§3, still the open
  question). `Styleguide::color(name)`/`.dimension(name)` are the whole lookup API.
- **`apps/anvil/anvil.styleguide.md`**: the actual token file — a Tokyo-Night-derived palette
  (`background`, `surface`, `border`, `accent`, `accent-warn`, `success`, `danger`, `text`,
  `text-muted`) plus a handful of unused-so-far dimension tokens (`radius`, `spacing-*`).
- **Terminal side**: `apps/anvil/src/main.rs`'s previously-hardcoded `ACCENT_BLUE`/`ACCENT_ORANGE`/
  `GREEN`/`RED`/`DIM` consts (and two raw hex-literal `with_style` calls) now resolve from this
  file via a `LazyLock<Styleguide>`, with the original hardcoded values kept as fallbacks.
- **AppKit side**: `escher-appkit` gained a minimal `Theme { background, accent, text }` (just
  three colors, deliberately not the full token set) on `AppKitSurface` — `set_theme()` paints the
  surface's own root background (new: `FlippedView` can now fill itself, previously painted
  nothing) and colors newly-created tab rows/labels/text fields. `escher_appkit::bevy` exposes this
  as a `ThemeState` resource; `apps/anvil` populates it from the same `Styleguide` instance the
  terminal side reads, so both surfaces are provably reading the same token source, not just
  visually similar by coincidence. Verified live: toolbar/tab-strip background is now the dark
  styleguide `background` color (previously transparent/system-default) and the active tab
  highlight uses the styleguide `accent` blue (previously `NSColor::selectedContentBackgroundColor`,
  a system color that wouldn't have matched the TUI in light mode or a non-default accent).
- **Not touched**: Bevy scene surface and webview *content* styling (arbitrary web pages aren't
  ours to restyle) — the human's original four-surfaces framing only fully applies to the two that
  are ours to paint (terminal, AppKit chrome). Buttons/bezels still use system AppKit rendering
  (NSButton's default bezel, not restyled). Dimension tokens are parsed but nothing consumes them
  yet. No hot-reload — theme is read once at startup.
