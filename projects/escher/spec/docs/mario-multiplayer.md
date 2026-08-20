# Running the mario example (with a friend, over LAN)

`escher-bevy`'s `examples/mario` is a small terminal jump-and-attack platformer — one square per
connected gamepad, permanent ghosts for every lost life, and optional cross-machine multiplayer.
This page covers getting it running solo, then hosting or joining a real multi-machine session.

## Requirements

- A gamepad. Controls are gamepad-only: left stick or d-pad to move, South to jump (again in the
  air for a double jump, or to wall-kick off a wall), East to attack (hold a trigger while
  attacking for a heavier hit), Start to open the pause menu.
- The `audio` feature, for this example's sound effects — see below.

## Running it locally

`examples/mario` needs `escher-bevy`'s `audio` feature (off by default for every other consumer of
that crate), so a bare `cargo run --example mario` fails with a "requires the features: `audio`"
error. From this repository's own `escher` project root, a `cargo mario` alias already has that
covered:

```sh
cargo mario
```

That's single-player: no relay reachable means no remote players show up, and everything else
(jumping, attacking, the ghosts) works exactly the same.

From outside this checkout (installed via `cargo install`, say), spell it out directly instead:

```sh
cargo run -p escher-bevy --example mario --features audio
```

## Hosting a session

One machine runs `--host`. This starts an embedded signaling server bound to every network
interface on this machine (not just loopback), so a peer elsewhere on the LAN can actually reach
it, and prints the LAN IP a peer needs plus a ready-to-copy `--connect` command:

```sh
cargo mario -- --host
```

## Joining as a peer

Everyone else runs `--connect <the host's LAN IP>` — printed by the host's own startup banner
above. This one flag derives both the relay signaling URL and the `sqld` persistence URL from that
one address, instead of needing either spelled out separately:

```sh
cargo mario -- --connect 192.168.1.23
```

If you don't have this checkout on the joining machine, `cargo install` from the branch directly
(no need to keep the source around afterward — see the cleanup note below):

```sh
cargo install --git https://github.com/brainbow-dx/source --branch escher/mario-platform-polish \
  -p escher-bevy --example mario --features audio
mario --connect 192.168.1.23
```

`cargo install --git` clones into `~/.cargo/git/` and does **not** delete that checkout
afterward — only the installed binary goes to `~/.cargo/bin`. To actually clear the source off a
machine you don't want it lingering on:

```sh
rm -rf ~/.cargo/git/checkouts/source-* ~/.cargo/git/db/source-*
```

### Overriding individual addresses

`--relay`/`--sqld`/`--port`/`--room`/`--name` are all still available if `--host`/`--connect`'s
derived addresses aren't right for a session's actual topology (the signaling server and `sqld`
living on different machines, say):

```sh
cargo mario -- --relay ws://192.168.1.23:9200/ws --sqld http://192.168.1.23:8081 --room our-session
```

## Known issues

- **Windows + Xbox controller**: input may be captured by a Windows-level gamepad-to-UI-navigation
  feature (Xbox Game Bar, or a background overlay like Steam) instead of reaching the game —
  symptoms are the gamepad being detected (a character appears) but button presses/stick movement
  shifting Windows UI focus rather than doing anything in-game. Try disabling Xbox Game Bar
  (Settings → Gaming → Xbox Game Bar) and quitting any controller-overlay app (Steam, GPU vendor
  overlays) first if you hit this.
- **Windows terminal**: run the actual `mario` binary from Windows Terminal, `cmd.exe`, or
  PowerShell, not Git Bash/MSYS's default `mintty` terminal — `crossterm`'s Windows backend talks
  to the native Win32 Console API directly, which `mintty` doesn't reliably provide.
