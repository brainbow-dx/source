# Anvil peer chat (Lorren ↔ Nasia, via sqld)

Status: proposed 2026-08-17. Priority #1 per the user directly — needed before the Escher/Anvil (Nasia) and Ethos/Atlas (Lorren) work-split tracks below can actually run in parallel day to day. Not started.

## What this is, and what it deliberately isn't

A minimal two-person chat inside Anvil's existing transcript UI, persisted through the `sqld` instance one of the two hosts already runs — not a new peer-connectivity subsystem. Per `atlas/spec/.agents/proposals/direct-peer-connections-threshold.md`, coordination-shaped data (chat, exactly this) is squarely sqld-sync's job; WebRTC is for the high-bandwidth/latency-sensitive data (tracing streams, video, drawing buffers) that this explicitly is not. Building this on WebRTC/Atlas signaling would be solving a harder problem than the one asked for.

## Current state (verified this session, not assumed)

`apps/anvil/src/main.rs`'s `persistence` module already does everything structurally needed except three things:

1. `SQLD_URL` is a hardcoded `"http://localhost:8081"` constant. Needs to be configurable (an env var is enough) so one person's Anvil can point at the *other* person's running `sqld` instead of its own. Per the user directly: this should resolve to a real peer address (their Tailscale address once that's set up), not stay a sqld-specific dev constant — but functionally, for this feature, "configurable via env var, defaulting to `localhost` unchanged" unblocks the feature today without waiting on the Tailscale-tagged-service work (`atlas/.../direct-peer-connections-threshold.md`'s "Provisioning" section, not started).
2. `Persistence::connect_inner` calls `database.sync()` exactly once, at connect. A live chat needs periodic re-sync (every 1-2s) so each side actually sees what the other just sent — not built yet. Given how little a two-person work chat's message volume actually is, resyncing and reloading the *whole* transcript on each poll is the right amount of engineering here, not incremental diffing (see the same threshold doc's discussion of when that would change).
3. The `messages` table has no author column — every `user` row is anonymous today. A shared chat needs to know *who* sent a message, not just that a message of kind `user` exists.

## Scope

- `SQLD_URL` → resolved from an env var (e.g. `ANVIL_SQLD_URL`), falling back to today's `http://localhost:8081` unchanged when unset — so a solo Anvil session (or CI, or anyone without a peer) keeps working exactly as it does today.
- A lightweight `author` identity: an env var (e.g. `ANVIL_USER`, defaulting to the OS username) is enough for two people — no login system, no identity service. Added as a real `author` column on `messages`, not smuggled into `content`.
- A background periodic sync: `database.sync()` on an interval, then a full `load_messages()` reload merged into the transcript. "Full reload" here means the app-level query, not the underlying libsql WAL replication, which is already incremental — see the threshold doc.
- Displaying `author` in the transcript so messages read as a real two-person conversation, not undifferentiated `User` bubbles.

## Explicitly out of scope for this pass

- Any real peer discovery/addressing mechanism (that's the Atlas peer-connectivity work, separate and not a dependency of this — env-var config is a deliberate, acceptable stand-in until it exists).
- Auth on the `sqld` connection itself — already noted as absent in the existing code; acceptable for two trusted collaborators on the same LAN/Tailscale network, not for anything wider.
- Anything beyond text chat (presence, typing indicators, read receipts) — not asked for.
