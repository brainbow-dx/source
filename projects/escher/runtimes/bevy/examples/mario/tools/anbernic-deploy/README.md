# Deploying mario to an Anbernic RG35XX-family device (muOS)

Thin wrappers around the generic tool at `Brainbow/tools/anbernic-deploy/` — see that directory's
README for what's actually happening (Docker cross-compile, muOS folder layout, troubleshooting)
and for the on-device path caveats that apply here too. This file only covers what's mario-specific.

## One-time setup

1. Install Docker Desktop (if not already): https://www.docker.com/products/docker-desktop
2. `chmod +x tools/anbernic-deploy/*.sh`

## One-time SD card setup

1. Insert the muOS SD card, find its mount point (`ls /Volumes/`).
2. `./tools/anbernic-deploy/prime-sdcard.sh /Volumes/<mount-name>`
3. Eject, insert into the device, boot it, note its IP under **Settings > Network** (confirm SSH
   is on).

## Every time you want to deploy

```bash
./tools/anbernic-deploy/build.sh
./tools/anbernic-deploy/deploy.sh <device-ip>
```

`build.sh` cross-compiles `escher-bevy --example mario --features audio` for
`aarch64-unknown-linux-gnu` inside Docker; `deploy.sh` pushes the resulting binary over SSH and
writes the Ports-menu launcher. No `--assets` flag anywhere — mario has no asset files (terminal
glyphs, procedurally synthesized sfx), so there's nothing to push besides the binary itself.

## What's genuinely unverified here

Everything below is real, reasoned engineering, not guesswork — but none of it has been tested
against actual Anbernic hardware yet (no device, no muOS SD card, no Docker daemon reachable from
the session that wrote this). Test in roughly this order:

1. **Does the cross-compile even succeed?** `cpal`/`rodio` (the `audio` feature) and gilrs-based
   gamepad support both link against real system libs (`libasound`, `libudev`) — the Dockerfile
   installs their aarch64 dev packages, but this hasn't been run end-to-end.
2. **Does the binary run at all on-device?** mario's Bevy `App` runs with `primary_window: None`
   on every non-Windows platform (confirmed by reading `main.rs`/`plugin.rs` directly, not
   assumed) — meaning the default terminal-only experience needs no display/GPU/compositor stack
   at all, only a TTY, ALSA, and evdev. This is the good news: the most likely failure mode
   (winit/Bevy's renderer choking on muOS's display stack) shouldn't even be reachable unless you
   deliberately open the on-demand Bevy scene window (`B` key) — untested whether the Ports menu
   itself hands the launched process a real TTY at all, though.
3. **Does audio/gamepad actually work?** Depends on `libasound`/`libudev` (or their eventual
   replacements) actually being present on muOS's own rootfs at runtime, not just at cross-link
   time — if missing, `deploy.sh`'s launcher redirects stdout/stderr to `log.txt` in the port's own
   folder, which is the first thing to check over SSH if the game doesn't start.

See `spec/.agents/changelog.md` for when this was built and why (part of a larger conversation
about getting a genuinely thin — input/display/audio + one binary, no desktop environment — build
running on this hardware; muOS itself is being used here purely for its already-working driver
stack during development, not as the intended final runtime).
