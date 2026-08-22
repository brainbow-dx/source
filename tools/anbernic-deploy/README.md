# anbernic-deploy

Generic tooling for cross-compiling a Rust binary to aarch64 and deploying it to an Anbernic
handheld running muOS, over Docker + SSH. Knows nothing about any one project — it's meant to be
called from a project's own `tools/anbernic-deploy/` wrapper scripts, the way
`runtimes/bevy/examples/mario/tools/anbernic-deploy/` calls it. See that directory for a working
example of the split.

## Why split this way

The cross-compile toolchain, the muOS Ports folder convention, and the SSH deploy mechanics are
the same for every project that wants to target this hardware. What differs per project is just
the Cargo package/example-or-bin name, feature flags, and whether there are assets to push. This
tool owns the first set; a project's own thin wrapper scripts own the second.

## One-time setup

1. Install Docker Desktop: https://www.docker.com/products/docker-desktop
2. `chmod +x` this directory's `*.sh` (a project's own wrapper scripts do this for you already).

No host Rust toolchain, no cross-linker install, no `brew install` — the Dockerfile bundles the
aarch64 GCC cross toolchain and the ALSA/udev headers `cpal`/`gilrs` need to link, and pulls in
the `aarch64-unknown-linux-gnu` Rust target itself on first build (cached after that in a named
Docker volume, so later builds don't redo it).

## The three scripts

- **`docker-build.sh`** — cross-compiles a package inside Docker.
  `--workspace <path> --package <pkg> (--example <name> | --bin <name>) [--features <list>] --out <path>`
- **`deploy.sh`** — pushes a built binary (+ optional assets) to a device over SSH, writes a launch
  script into muOS's Ports menu.
  `<device-ip> --bin <path> --slug <name> --launch-name <"Display Name"> [--assets <dir>] [--user root] [--remote-base /mnt/mmc]`
- **`prime-sdcard.sh`** — one-time SD card bootstrap, run from the Mac before the device has ever
  booted with SSH reachable, so the Ports menu shows a real entry immediately.
  `<sdcard-mount-path> --slug <name> --bin-name <name> --launch-name <"Display Name">`

`lib.sh` holds what's shared between them (muOS's own `ports/`/`roms/PORTS/` folder convention,
the launcher script template, logging helpers) — not meant to be run directly.

## muOS folder convention (sourced, not guessed)

A port's binary and assets live at `<sdcard>/ports/<slug>/`; the launcher `.sh` that muOS's own
Ports menu scans for lives separately, at `<sdcard>/roms/PORTS/<Display Name>.sh`. Confirmed
against muOS's own docs and community sources, not assumed. What's genuinely **not** confirmed
from here: the on-device absolute path the SD card mounts at (`deploy.sh`/`prime-sdcard.sh` both
default to `/mnt/mmc`, a common convention for this device family, but not independently verified
against this specific hardware — `--remote-base` overrides it), and the exact conventions muOS's
own Application Runner / `mux_launch.sh` wrapper expects for CPU-governor and SDL environment
setup. The launcher this tool writes is a plain, dependency-free shell script that just `cd`s in
and `exec`s the binary — if a game doesn't launch from the Ports menu even though running that
same script by hand over SSH works, that gap is the first thing to check against muOS's current
docs.

## Troubleshooting

| Problem | Fix |
|---|---|
| `docker: command not found` | Install/start Docker Desktop |
| `docker build`/`docker run` hangs or times out | Docker Desktop may not be fully started — open the app, wait for it to report "running," retry |
| `ssh: connect to host ... Connection refused` | SSH isn't enabled on-device — check Settings > Network |
| `mkdir` over SSH fails with "No such file or directory" | `--remote-base` is probably wrong for this device — `ssh <user>@<ip> 'ls /mnt/mmc'` (or wherever the card actually mounts) to find the real path |
| First build very slow | Expected — the Docker image and full crate compile happen once; later builds reuse the cached registry/target volumes |
| Game launches to a blank screen or doesn't launch at all | See the "not confirmed" note above re: muOS's own launcher conventions |
| Binary runs but crashes immediately | Missing `.so` deps on-device — check `log.txt` in the port's own folder (the launcher redirects stdout/stderr there); may need to drop libs alongside the binary and point `LD_LIBRARY_PATH` at them in the launcher |
