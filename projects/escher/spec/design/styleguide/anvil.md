---
type: styleguide
colors:
  background: "#202020"
  chrome: "#262a30"
  surface: "#333844"
  control-hover: "#3d4350"
  border: "#40454f"
  accent: "#61afef"
  accent-warn: "#cc9a4d"
  success: "#6a9955"
  danger: "#d9534f"
  text: "#e8e8e8"
  text-muted: "#8a8a8a"
dimensions:
  radius: 8
  spacing-sm: 4
  spacing-md: 8
  spacing-lg: 16
text:
  ui: 15
  body: 13
---

# Anvil styleguide

Shared token source for Anvil's terminal UI and native AppKit browser chrome (toolbar, tab strip) — one palette, so both surfaces read as the same app instead of two unrelated ones stuck together. Loaded once at startup via `escher-styleguide`; see that crate for the parser/lookup API and `spec/.agents/proposals/styleguide-frontmatter.md` for the long-term W3C-token-format design this is a minimal, unblocked first slice of (aliasing, typed `$value`/`$type` tokens, and component-dependency declarations are all still deferred).

`text.ui`/`text.body` are AppKit-only (button glyphs vs. label/field body text) — the terminal surface has no equivalent: its font size/family belong to whatever terminal emulator the user is actually running in, entirely outside this app's control. Terminal text styling only ever draws from `colors` (see `apps/anvil/src/main.rs`'s `STYLEGUIDE`-backed color statics).

`chrome`/`surface`/`control-hover`/`border` are a deliberate layered stack for the AppKit browser chrome specifically, not just three more colors: `background` is the page/terminal content plane; `chrome` sits one step above it as the toolbar/tab-strip's own surface (so chrome visibly reads as chrome instead of blending into whatever page happens to be loaded); `surface` is another step up, for a control that should visibly lift off the chrome it sits in (the address bar, a hovered/pressed toolbar button); `control-hover` is `surface`'s own hover/press state, one step up again. `border` is a hairline separator's color, not a fill — used at chrome/content and chrome/tab-strip seams. The terminal surface has no equivalent concept of layered chrome, so it only ever reads `background`/`surface`/`text`/the accent family, same as before this stack existed.