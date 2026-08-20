---
type: styleguide
colors:
  background: "#1a1b26"
  surface: "#1f2335"
  border: "#3b4261"
  accent: "#7aa2f7"
  accent-warn: "#e0af68"
  success: "#9ece6a"
  danger: "#f7768e"
  text: "#c0caf5"
  text-muted: "#565f89"
dimensions:
  radius: 6
  spacing-sm: 4
  spacing-md: 8
  spacing-lg: 16
text:
  ui: 15
  body: 13
---

# Anvil styleguide

Shared token source for Anvil's terminal UI and native AppKit browser chrome (toolbar, tab
strip) — one palette, so both surfaces read as the same app instead of two unrelated ones stuck
together. Loaded once at startup via `escher-styleguide`; see that crate for the parser/lookup
API and `spec/.agents/proposals/styleguide-frontmatter.md` for the long-term W3C-token-format
design this is a minimal, unblocked first slice of (aliasing, typed `$value`/`$type` tokens, and
component-dependency declarations are all still deferred).

`text.ui`/`text.body` are AppKit-only (button glyphs vs. label/field body text) — the terminal
surface has no equivalent: its font size/family belong to whatever terminal emulator the user is
actually running in, entirely outside this app's control. Terminal text styling only ever draws
from `colors` (see `apps/anvil/src/main.rs`'s `STYLEGUIDE`-backed color statics).
