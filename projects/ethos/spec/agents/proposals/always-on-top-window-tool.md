# Always-on-top window helper — plan (not started)

Status: proposed, 2026-08-15. Written by an agent fork per human request ("no tool install [on this machine] and we should write this into our toolset anyway"), for the root agent to pick up first next session. Not started — no code, no crate scaffolding.

## The ask

Launch the Unity Editor from a Deno script and force its window always-on-top (floating above every other window), so it stays visible while the developer works in another app. First concrete consumer: `projects/aby/scripts/open-unity-sandbox.sh` (currently a plain bash loop with no window management at all) — but scoped as a reusable tool, not an Aby-only hack.

## Grounding: what's actually true here, verified live

- **No window-leveling tool is installed on this machine.** Checked directly: `yabai`, `skhd` not on `$PATH`; no yabai/Amethyst/Rectangle in `/Applications`. Confirmed by the human before this spec was written — the ask is explicitly to build this in, not `brew install` something.
- **Launching Unity from Deno is not the hard part.** A plain `Deno.Command`/`$` subprocess spawn (`open -a Unity.app --args -projectPath ...`, or the Unity binary directly) works exactly the way this session already launched Unity manually for Smash & Stab verification. No blocker here.
- **"Always on top" for an *external* process's window has no public macOS API.** `escher-bevy`/`apps/anvil` already use `winit::window::WindowLevel::AlwaysOnTop` (`runtimes/bevy/src/legacy/window.rs`, `apps/anvil/src/main.rs:803`) — but that only works because winit/AppKit *creates* the window itself and can call `-[NSWindow setLevel:]` on its own object. Unity is a separate, unrelated process; nothing in this repo owns its `NSWindow`. There is no documented Apple API for one process to set another, unrelated process's window level. Tools like `yabai` that do this rely on **private, undocumented SkyLight/CGS framework symbols** (`SLSSetWindowLevel` / `CGSSetWindowLevel`) — not the public Cocoa API surface. That's also why `yabai` needs elevated Accessibility permission and, for some of its features, System Integrity Protection partially disabled.

## Two tiers — pick one to actually build, don't blend them silently

**Tier A — public API only, "keeps regaining focus" (not true floating)** Poll for Unity's PID/frontmost state and call `NSRunningApplication.activate` (or `osascript`/System Events `set frontmost`) whenever something else steals focus. Fully supported, no private APIs, no extra permissions beyond normal automation consent. Real limitation: this is "keeps popping back to the front," not "floats above everything at all times" — a window you briefly click on the desktop *will* show, then Unity snaps back on the next poll tick. Cheap (a few hours), low risk, but may not satisfy "always on top" as actually meant.

**Tier B — real floating window level, via private APIs (what `yabai` does)** A small native helper resolves Unity's `CGWindowID` via the *public* `CGWindowListCopyWindowInfo` (cross-referenced by owning PID — this part is fully public/documented), then calls the private SkyLight window-level setter on it. This is genuinely "float above everything," matching what the ask actually wants. Real costs: undocumented API, no stability guarantee across macOS versions, needs Accessibility permission grant on first run, and needs periodic re-verification that the private symbols still resolve after an OS update. This is the same tradeoff `yabai` itself accepts — reasonable for an internal dev tool, not something to ship to end users without a fallback.

**Recommendation: build Tier B, with Tier A as an automatic fallback** if the private symbols fail to resolve at runtime (log a warning, degrade to polling-refocus rather than hard-erroring) — the human's phrasing ("build it into our toolset") reads as wanting the real behavior, not the approximation, but degrading gracefully instead of just failing on some future macOS is cheap enough to include from the start.

## Where this lives

`projects/ethos/services/dev` (`@ethos/dev`) is the existing cross-project Deno tooling package — `src/shell.ts` already provides the `$`/`ManagedProcess` helpers that `escher/runtimes/web/scripts /dev.tsx` and others import today. This is the natural home for a new `src/window.ts` module, so any project's Deno scripts (Aby's Unity launcher, escher's own dev scripts, future ones) can pull it in the same way they already pull in `$`. `@ethos/dev` already has both a Rust side (`src/ lib.rs`, `src/main.rs`) and a Deno side — matches needing a small native helper binary plus a thin TS wrapper around it.

## Concrete plan

1. **Spike first, before committing to Tier B**: confirm the private SkyLight symbols (`SLSSetWindowLevel` or `CGSSetWindowLevel`, exact symbol name needs checking against the currently-installed macOS version) actually resolve and work from a throwaway `dlopen`/`dlsym` test. This is the one real unknown — if it doesn't pan out cleanly, fall back to Tier A only and say so, rather than sinking more time chasing a private API that's stopped working.
2. Small Rust binary (new bin target in `ethos/services/dev`, or a sibling crate) using `objc2` + Core Graphics — same dependency shape `escher-appkit` already uses in this repo, so the pattern is proven here, not novel. Takes a PID, resolves its `CGWindowID`(s) via `CGWindowListCopyWindowInfo`, sets floating level via the private symbol from step 1. Tier-A fallback (poll + `NSRunningApplication.activate`) lives in the same binary, selected automatically if step 1's symbol lookup fails.
3. `src/window.ts` in `@ethos/dev`: `setAlwaysOnTop(pid: number): Promise<{ tier: "floating" | "refocus-poll" }>` — spawns the native helper, surfaces which tier actually engaged so callers (and the developer) know which behavior they're getting.
4. Update `projects/aby/scripts/open-unity-sandbox.sh` (or port it to a `.tsx` Deno script, since `@ethos/dev` is Deno-side) to launch Unity and call `setAlwaysOnTop` on the resulting PID.
5. Document the macOS-version fragility risk directly in `@ethos/dev`'s README — this is a "may need a fix after an OS update" tool, not a "set and forget" one, and that should be visible to whoever touches it next, not just written down here.

## Explicitly out of scope for this pass

- Cross-platform (Windows/Linux) always-on-top — macOS only, matching every other native-window concern in this repo so far (`escher-appkit` is macOS-only today too).
- Folding this into Hudd. Hudd (`apps/hudd`) is the eventual always-on-top overlay *daemon* for Escher's own surfaces, and doesn't exist yet (still a 14-line stub, see `spec/ROADMAP.md` M7). This tool is a narrow, standalone utility for floating an *arbitrary external app's* window (Unity), built now because it's needed now — not a first slice of Hudd, even though Hudd will eventually want the same window-leveling primitive internally.
