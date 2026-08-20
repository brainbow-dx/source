# Proposal: finishing the style & events systems, and adding animation

Status: **Style (§1) implemented** — see `spec/.agents/changelog.md`, "Style system v1". **Events (§2) and Animation (§3) not yet implemented.** Written per the human's request to scope this before writing code, and per `AGENTS.md`'s default (proposals belong here for human review).

Scope was narrowed with the human up front, since "full CSS spec" / "the events spec" / "animation" are each independently huge:

- **Style**: a practical, terminal-appropriate subset — finish the box model, flex layout, color, and typography that are already sketched out; skip selectors/cascade/stylesheets/media-queries/viewport-units, which don't have an obvious meaning for a Rust builder-API library targeting five different runtimes (terminal, web, bevy, unity, unreal).
- **Events**: build out the existing `ui-events` dependency (`packages/core/src/event.rs` already re-exports it) — real target/bubble dispatch, not a new spec from scratch.
- **Animation**: CSS-transition-style (a value smoothly interpolates over a duration/easing when it changes), scoped to what's actually achievable given the framework's current architecture (see below).

Every finding below was verified by reading the actual current code (`packages/core/src/style.rs`, `packages/core/src/event.rs`, `packages/core/src/scaffold.rs`, `runtimes/terminal/src/surface.rs`) and the `ui-events` crate's own source, not assumed.

---

## 1. Style system

### What's actually there today (with two real bugs found)

| Property | Status |
|---|---|
| `Margin` | Applied at the *layout* level (spaces out child slots) — but only as one uniform value; `Edge::Top/Right/Bottom/Left` is parsed but never distinguished, so `Margin::top(2)` and `Margin::left(2)` currently behave identically. There's also a second, dead `Property::Margin` arm in the content-rendering loop that just logs `"Margin not yet implemented"` and does nothing — confusing leftover, not doing anything the layout-level handling isn't already doing. |
| `Padding` | **Bug**: `into_padding()` correctly builds a per-edge `ratatui::widgets::Block` (`runtimes/terminal/src/surface.rs:495`), but that `Block` is never rendered — `frame.render_widget(widget, *content_area)` draws the `Paragraph` directly, the `block` variable is simply discarded. Padding has **zero visual effect** right now despite the conversion logic looking complete. |
| `Gap` | Works — applied as flex spacing. |
| `FlexDirection` | Works — Row/Column. |
| `Flex` (grow weight) | Parsed into the style sheet, **never read**. Every child without an explicit `Size` gets `Constraint::Min(0)` regardless of its declared `Flex` weight, so multiple flex children always split space evenly — no actual flex-grow behavior. |
| `FontStyle` | Only `Italic`/`Normal`. Ratatui's own `Style` already natively supports bold, underline, strikethrough, dim — none of that is exposed. |
| `ContentColor` / `BackgroundColor` | Work, applied to the `Paragraph`'s style (confirmed — this is how the recent `assistant.rs` overlay's background paints correctly). Alpha channel is parsed (`LinSrgba`) but never used — ratatui's `Color::Rgb` has no alpha, so alpha is currently decorative only. |
| `Border` | Works, including color (fixed earlier this session). |
| `Heading` | Enum exists (`Primary`/`Secondary`/`H1`–`H6`), **completely unused** anywhere in rendering. |
| `ScrollPosition` | **Completely inert** — confirmed via grep, nothing in the terminal runtime reads it. |
| `OverlayInset` | Added this session, works (overlay positioning only). |

### Proposed v1 scope

1. **Fix Padding** — stop building a discarded `Block`; either inset `content_area` directly by the padding amount before laying out the `Paragraph`, or actually pass `.block(block)` to the `Paragraph` (recommend the manual inset — using `Block` would also draw a border if one were ever added to it, which would conflict with the existing separate Border-rendering pass earlier in `render()`).
2. **Fix Margin** to be genuinely per-edge, and delete the dead "not yet implemented" branch.
3. **Wire up `Flex`** — when a child has an explicit `Flex` weight, use `Constraint::Fill(weight)` instead of always `Constraint::Min(0)`, so flex-grow actually works.
4. **Extend `FontStyle`** (or introduce a `FontWeight`/`TextDecoration` set alongside it) to cover Bold, Underline, Strikethrough, Dim — cheap, since ratatui supports all of them natively already.
5. **Decide `Heading` and `ScrollPosition`'s fate** — either give them real behavior or explicitly document them as reserved/unimplemented (not leave them silently inert, which is what invites bugs like the Padding one above). Recommend: implement `Heading` as a small font-weight/size-hint preset per variant (cheap); implement `ScrollPosition` as a real scroll offset for `Body`-style overflow content (moderate — needs the rendered content to be larger than its viewport rect and the buffer draw offset by the scroll amount).
6. **Add `Overflow`** (Visible/Hidden/Scroll — ties into `ScrollPosition`) and **`TextAlign`** (Left/Center/Right — ratatui's `Paragraph::alignment()` already supports this natively, cheap).
7. **Explicitly out of scope**: CSS selectors/cascade/specificity, a stylesheet/CSS-text parser, media queries, viewport units, CSS Grid, `:hover`/`:focus` pseudo-classes (depends on the Events work below — worth revisiting after).

---

## 2. Events system

### What's actually there today

- `ui-events` (the dependency) is **explicitly data-types-only** — straight from its own crate docs: *"This crate is intentionally focused on data structures — it does not open windows or read events."* It gives us a W3C-UI-Events-modeled vocabulary (`PointerEvent`, `PointerButtonEvent`, `ScrollDelta`, keyboard types), but propagation/dispatch is entirely Escher's own responsibility — "building it out" means building Escher's dispatch logic, not configuring the dependency.
- `Scaffold::dispatch<E>` (`packages/core/src/scaffold.rs`) currently **broadcasts**: it recurses into every child unconditionally, then runs its own handlers. There is no concept of a specific event *target*, no hit-testing, and no capture/bubble phase — every matching handler in the *entire tree* fires on every event of that type. That's workable-ish for keyboard input (arguably global/focus-independent today), but wrong for pointer input, which needs to hit-test to one target and then walk just that node's ancestors.
- Mouse/pointer input **isn't wired up at all** in the terminal runtime: `CrosstermEvent::Mouse(..)` is a bare `// TODO: Handle events!` stub, despite `ui_events::pointer::PointerEvent` already being available and re-exported.
- No `stopPropagation`-equivalent: `EventHandler::call` returns nothing, so a handler can't halt further dispatch.

### Proposed v1 scope

1. **Retain per-frame `Rect`s** — `render()` computes each node's `content_area`/`slot_area` today and throws it away; hit-testing needs these kept somewhere addressable for the current frame.
2. **Real hit-testing + target/bubble dispatch for pointer events** — find the deepest node under the pointer position, then walk its ancestor chain firing handlers target-to-root. This needs `Scaffold` to know its parent chain, which doesn't exist today (traversal is strictly top-down via `slots`) — this is the biggest structural addition in this whole proposal.
3. **Wire `CrosstermEvent::Mouse` → `ui_events::pointer::PointerEvent`**, mirroring the existing `unpack_keyboard_event` conversion, so terminal apps can register `.handle::<PointerEvent>(...)`.
4. **Add stop-propagation** — change `EventHandler::call`'s signature to let a handler signal "don't keep bubbling."
5. **Leave keyboard dispatch's current broadcast behavior alone for v1** — focus-tracking ("which node currently has keyboard focus") is its own sizable feature; scoping it into this pass would balloon it further.
6. **Explicitly out of scope**: full `addEventListener`-style API (capture-phase opt-in, `once`, passive listeners), synthetic event replay, focus management/tab order.

---

## 3. Animation

### The constraint that shapes everything here

Scaffold trees are **rebuilt from scratch every frame** — arena-allocated fresh via `Herd::get()` on each `draw()` call, with no retained identity or diffing between frames. This isn't a gap I'm inventing: it's explicitly flagged as future work in the framework's own code (`runtimes/terminal/src/surface.rs`, `Surface::draw`): *"Optionally (via cfg) apply retained-mode rules: Find and apply Node ids... Nodes with changes should be marked."* That TODO is exactly the prerequisite "true" CSS transitions need — the browser can animate `background-color` because it retains the DOM node and diffs old-vs-new computed style. Escher currently has neither the retained node nor the diff.

### Two paths

- **(A) Solve retained-mode diffing first, then automatic transitions on top.** This is the "real" experience — just change a style value and it animates. It's also, on its own, roughly as large as the rest of this proposal combined (stable node identity across frames, a diff pass, animation state keyed to that identity).
- **(B) Explicit `Transition<T>` helper, app-managed.** A small library type callers hold in their own persistent state (the same `Arc<RwLock<...>>` pattern every example already uses for things like `expanded`/`user_input`), read once per frame to produce the interpolated value they pass into `.style(...)`. Gets the CSS-transition *feel* — smooth interpolation, duration, easing — without requiring the framework to solve reconciliation first.

**Recommend (B) for v1.** It's honest about what's achievable without a much bigger prerequisite, ships something genuinely usable now, and doesn't foreclose (A) later — a future automatic system could reuse the same `Lerp`/`Easing` primitives underneath.

### Proposed v1 scope

- New `Transition<T: Lerp>` type (likely `packages/core/src/animation.rs`): `Transition::new(initial)`, `.set_target(value, duration, easing)`, `.value_at(Instant) -> T`.
- Small `Easing` enum: Linear, EaseIn, EaseOut, EaseInOut.
- `Lerp` implemented for the style primitives worth animating: `Unit`, `Color`/`ContentColor`/`BackgroundColor` (channel-wise RGB), `Value`'s `Px` variant (interpolating to/from `Auto`/`Fill`/`Percent` doesn't have a sensible meaning, so those stay instant).
- Explicitly out of scope for v1: automatic/implicit transitions triggered by a style change (needs (A)), keyframe sequences (`@keyframes`-equivalent), iteration-count/fill-mode semantics.

---

## Sequencing

Even at this scoped-down size, these are three substantial, largely-independent pieces of work — Style touches most of `surface.rs`'s render function; Events needs a structural addition to `Scaffold` (parent links) that doesn't exist today; Animation is new from nothing. Recommend tackling them as three separate reviewable passes rather than one large patch, in whatever order is most useful — Style is probably the lowest-risk starting point since it's fixing/extending things that already mostly exist, versus Events' structural change or Animation's from-scratch module.
