# Proposal: app-level view/routing for Scaffold apps

Status: **not implemented — proposal only**, per `AGENTS.md`'s default. Written up from a design discussion with the human on 2026-08-15; no code was written for this yet.

## The ask

Apps need a way to manage UI state transitions between large, repeated "views" — the same problem React Router (and similar) solve, but the human explicitly wants it handled differently: most frameworks solve this at the *component* level (a wrapper/router component defines routing for its own subtree, nested arbitrarily deep). The human wants this handled at the **app level** instead, with simple hooks exposed to app builders for composing more complex routing/view behavior on top.

## Why app-level fits this framework specifically

A `Scaffold` tree is rebuilt fresh every draw call from a plain Rust closure the app supplies (no retained component instances, no lifecycle hooks a "router component" could hang off of the way a React component can). App state already lives in Bevy ECS `Resource`s, with `Message`s driving state transitions and a top-level system matching on that state to decide what to compose — this is exactly the shape `apps/anvil`'s `BrowserState`/`ToolbarEvent`/`TabStripEvent` already use (see `escher/spec/.agents/changelog.md`'s 2026-08-15 entry, and `escher_appkit::bevy` for the concrete pattern). Routing fits into this same shape directly; inventing a separate component-level mechanism would fight the framework's own architecture rather than build on it.

## Proposed design

- A generic `ViewStack<V>` type (likely `escher-core` or `escher-chalk`; `V` is the app's own view enum) as a Bevy `Resource`, holding the current view plus a history stack for back/forward navigation.
- A `Navigate<V>` `Message` (`Push(V)` / `Pop` / `Replace(V)`) apps fire to change view — same shape as `ToolbarEvent`/`TabStripEvent`.
- A small system, supplied by the helper, that drains `Navigate<V>` messages and updates `ViewStack<V>` accordingly.
- The app's own top-level `Scaffold`-building system reads `Res<ViewStack<V>>` and matches on `.current()` to decide which composition to call — plain Rust `match`, no new runtime dispatch mechanism needed.

## Tradeoff (accepted, flagged to the human, not yet re-litigated)

App-level routing centralizes control — one source of truth for "what's on screen," trivial back/forward and (eventually) deep-linking, easy to reason about. It loses the free encapsulation component-level routing gives independent, reusable subtrees: a reusable "tabbed settings panel" component that wants private internal navigation has to be handed its own separate `Resource` by the parent app rather than owning that state invisibly. Acceptable given the framework's existing resource-driven architecture, but worth remembering if a future composition wants that kind of self-contained internal routing — it will need its own `ViewStack<V>` instance, explicitly wired by whatever app embeds it, not something it can set up privately.

## Explicitly out of scope for v1

- Deep-linking / URL-equivalent serialization of a view stack (there's no URL bar concept outside the browser-specific tab strip built this session — a generic one would need its own design).
- Transition animations between views (depends on the separate animation proposal, `style-events-animation.md` §3, if that ever lands).
- Nested/scoped view stacks as a first-class framework feature (the tradeoff above describes the workaround — a manually-wired second `ViewStack<V>` — not a built-in nesting mechanism).

## Open questions for the human

None blocking — this is ready to scope into concrete tasks whenever the human wants it implemented. One thing worth deciding before implementation: should `ViewStack<V>` live in `escher-core` (framework-level primitive) or `escher-chalk` (reusable-but-optional component)? Leaning `escher-chalk`, since routing is an opinionated *pattern* an app builder opts into, not a `Scaffold`-rendering primitive every surface needs to know about — same reasoning that put `toolbar` there instead of `escher-core`.
