# Vision: universal drawing/annotation tooling across every edit-mode surface

Status: **vision/direction only — not a scoped proposal, no code, nothing to schedule yet.** Captured verbatim from the human on 2026-08-15 specifically so it isn't lost before it can drive later design work. Deliberately thin: this document exists to record intent, not to design the feature.

## The ask

Escher is building toward a full stack of apps — an embedded desktop app, `apps/hudd`, `apps/anvil`, and whatever other editor tools come out of that work — that together form a layered development experience built into the user's own OS, where building apps feels just like using apps. Every controlled surface across that stack that has edit-mode features is meant to expose the same drawing capability to the developer: the ability to draw *on* any surface in the entire stack, using tooling that's familiar and consistent everywhere it appears — not a bespoke per-app feature reinvented each time.

## Relationship to existing work

This is **not** the same thing as `[[scaffold-edit-mode]]` (this same folder) — that proposal covers *structural* editing (insertion/selection/deletion of Scaffold nodes, round-tripped to source via an Ethos-backed dialect). This vision is about **drawing/annotation** specifically: a freeform tool layered on top of whatever edit mode a surface already has, for the same reason a design tool has both "select/move objects" and "draw" as distinct but coexisting modes. The two are related (both are edit-mode capabilities that need to exist per-surface) but are not the same mechanism and shouldn't be conflated when either gets actually scoped.

Also relevant: `[[project_hudd_vision]]` and `[[project_desktop_vision]]` (both memory, not yet built) — this vision applies to both once they exist, not just to `apps/anvil` which is further along today.

## What's explicitly NOT decided here

Nothing about *how* — no tool architecture, no decision about whether this reuses/extends the Scaffold-editing selection machinery, no decision about output format (does a drawing become its own artifact, an annotation layer, or literal generated Scaffold nodes?), no per-surface feasibility check (a terminal surface "drawing" means something very different from an AppKit or webview surface). All of that is future scoping work, once the stack itself is further along and there's a concrete surface to prototype against.

## Why this is being logged now

The human asked to capture this specifically so it survives to inform later work, not because it's actionable today — `apps/hudd` and the desktop app don't exist yet, and even `apps/anvil`'s own edit mode is blocked on Ethos (see `[[scaffold-edit-mode]]`). Treat this as a design constraint to keep in mind once any of those become real: whatever drawing tool gets built for the first surface should be built with the expectation that it needs to generalize to every other surface in the stack, not as a one-off.
