//! A small terminal game proving Escher's terminal-plus-Bevy stack for game dev: a jump-and-attack
//! platformer, one square per connected gamepad, with permanent ghosts for every lost life and
//! optional cross-machine multiplayer over `atlas-relay`.
//!
//! Controls (gamepad only): left stick or d-pad to move, South to jump (again in the air for a
//! double jump, or to wall-kick off a wall), East to attack, Start to open the pause menu. Ctrl+C
//! or Escape quits. `B` (keyboard) opens the same running game in a real Bevy scene window
//! alongside the terminal one; see `scene.rs`.

mod ghosts;
mod persistence;
mod physics;
mod relay;
mod render;
mod scene;
mod sfx;

use std::collections::HashSet;
use std::io::Stdout;
use std::io::Write as _;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use bevy::app::App;
use bevy::app::AppExit;
use bevy::app::Last;
use bevy::app::Plugin;
use bevy::app::PreUpdate;
use bevy::app::Startup;
use bevy::app::Update;
use bevy::ecs::entity::Entity;
use bevy::ecs::message::MessageReader;
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::Commands;
use bevy::ecs::system::Res;
use bevy::ecs::system::ResMut;
use bevy::window::ExitCondition;
use bevy::winit::WinitSettings;

use clap::Parser;

use color_eyre::Result;

use parking_lot::RwLock;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;

use escher_core::element::Body;
use escher_core::element::Header;
use escher_core::style::FlexDirection;
use escher_core::style::Size;

use escher_core::event::keyboard::Code;
use escher_core::event::keyboard::KeyState;
use escher_core::event::keyboard::KeyboardEvent;

use escher_terminal::app::TerminalAction;
use escher_terminal::surface::TerminalSurface;

use escher_bevy::terminal::spawn_input_watcher;
use escher_bevy::EscherBevyConfig;
use escher_bevy::EscherBevyPlugin;

use physics::MarioState;

/// Fixed namespace for deriving this example's own identity UUID from `--name`. Distinct from
/// Anvil's own namespace: this is separate persisted state, not shared with it.
const IDENTITY_NAMESPACE: uuid::Uuid = uuid::uuid!("2c9b6a3e-2f0e-4f36-9b7a-2b6a6a8e6f1a");

const HEADER_HEIGHT: u16 = 1;

const BACKDROP_TEXT: &str =
    "Escher terminal game example.  Left stick: move.  South: jump (again in air for a double jump).  East: attack.  Start: pause menu.  Ctrl+C: quit.";

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The `atlas-relay` WebSocket this instance signals position sync through. No relay reachable
    /// at this address just means no remote players show up: local play keeps working regardless.
    #[arg(long, default_value = "ws://127.0.0.1:9200/ws")]
    relay: String,

    /// The `atlas-relay` room this instance's positions are broadcast and received in. Every
    /// instance in the same session needs to agree on this to see each other move.
    #[arg(long, default_value = "mario")]
    room: String,

    /// This instance's display name, shown to other players and recorded against any ghosts it
    /// leaves. Defaults to this machine's hostname.
    #[arg(long)]
    name: Option<String>,
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    color_eyre::install()?;

    // `color_eyre`'s own panic hook only writes to stderr, which is not a reliable place for a
    // message to survive once `terminal_startup` puts this process into raw mode/the alternate
    // screen: whatever the terminal was showing (including a panic that happened after that
    // point) is gone the moment the process exits without explicitly leaving the alternate
    // screen first, which a panic never gets the chance to do. This is the same fix
    // `apps/anvil` already needed for its own identical symptom (a crash that reads as a
    // silent, message-free exit). Chaining a second hook that also appends the panic message
    // and a backtrace to a plain file gives a real crash trail independent of whatever the
    // terminal itself was doing.
    let default_panic_hook = std::panic::take_hook();
    let panic_log_path = std::env::temp_dir().join(format!("escher-mario-{}", std::process::id())).join("panic.log");
    std::panic::set_hook(Box::new(move |info| {
        if let Some(parent) = panic_log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&panic_log_path) {
            let _ = writeln!(file, "{info}\n{}", std::backtrace::Backtrace::force_capture());
        }
        default_panic_hook(info);
    }));

    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();

    let name = args.name.clone().unwrap_or_else(|| hostname::get().ok().and_then(|name| name.into_string().ok()).unwrap_or_else(|| "player".to_string()));
    let identity_uuid = uuid::Uuid::new_v5(&IDENTITY_NAMESPACE, name.as_bytes());

    // Multi-threaded: the background persistence and relay tasks need genuine progress
    // independent of the render loop's own occasional blocking calls.
    let runtime = Arc::new(tokio::runtime::Builder::new_multi_thread().enable_all().build()?);

    let state = GameState::new(runtime.clone(), name, identity_uuid);
    persistence::spawn_connect_persistence(state.clone());
    relay::spawn(
        runtime.handle().clone(),
        args.relay.clone(),
        args.room.clone(),
        state.local_mario_snapshot.clone(),
        state.remote_mario.clone(),
        state.outgoing_combat_events.clone(),
        state.incoming_combat_events.clone(),
    );

    App::new()
        .add_plugins(EscherBevyPlugin::new(
            EscherBevyConfig::default()
                .with_window_title("Escher Mario Example")
                .with_spawn_primary_window(false)
                .with_spawn_terminal_plugin(false)
                // No window exists at all in the terminal-only case (only `scene.rs`'s on-demand
                // `B`-toggle ever opens one). Bevy's default `OnAllClosed` exit condition would
                // otherwise fire immediately since a window count of zero also counts as "all
                // closed." `terminal_exit`/the signal watcher already send the real `AppExit`.
                .with_exit_condition(ExitCondition::DontExit),
        ))
        // `EscherBevyPlugin` defaults to `WinitSettings::desktop_app()`, a power-saving mode that
        // only ticks the schedule every 5s while idle (or once a second, from
        // `spawn_input_watcher`'s own heartbeat wake; see its doc comment). That's exactly the
        // "gets stopped up"/1fps symptom this game hit before, back when it lived in Anvil as
        // `Page::Mario`: the original fix there was `sync_winit_update_mode_for_mario`, switching
        // to continuous `WinitSettings::game()` while that page was active. This example has no
        // other page to switch away from; it's always "the game." So it just runs in `game()`
        // mode the whole time, restoring the same continuous, vsync-paced tick the original had.
        .insert_resource(WinitSettings::game())
        .insert_resource(state)
        .add_plugins(MarioTerminalPlugin)
        .run();

    Ok(ExitCode::SUCCESS)
}

/// Everything the game's systems and its background tasks (persistence, relay) share. Every field
/// is already cheaply cloneable (an `Arc`, a `Copy` type, or a small `String`), so this whole
/// resource derives `Clone` rather than being individually destructured for each spawned task.
#[derive(Resource, Clone)]
pub struct GameState {
    pub mario: Arc<RwLock<Vec<(Entity, String, MarioState)>>>,
    pub visible_gamepad_candidates: Arc<RwLock<Vec<String>>>,
    pub gamepad_owned_by_me: Arc<RwLock<HashSet<String>>>,
    pub connected_players: Arc<RwLock<Vec<(String, String, (u8, u8, u8))>>>,
    pub local_mario_snapshot: Arc<RwLock<Vec<relay::PositionPacket>>>,
    pub remote_mario: relay::RemoteMarioTable,
    /// Hits this instance's own local attackers landed on a remote-tracked target, waiting to go
    /// out over the reliable combat channel. Pushed to by `physics::update_mario_physics`, drained
    /// by `relay`'s own combat send loop.
    pub outgoing_combat_events: Arc<RwLock<Vec<relay::CombatEvent>>>,
    /// Combat events landed on this instance's own local players by a remote attacker, waiting to
    /// be applied. Pushed to by `relay`'s combat receive loop, drained once per physics tick.
    pub incoming_combat_events: Arc<RwLock<Vec<relay::CombatEvent>>>,
    /// Cached wrapped backdrop rows, recomputed only when the terminal width actually changes.
    mario_wrap_cache: Arc<RwLock<Option<(u16, Arc<Vec<String>>)>>>,
    pub ghosts: Arc<RwLock<Vec<ghosts::GhostEntry>>>,
    pub cheat_menu_open: Arc<RwLock<bool>>,
    pub cheat_menu_selected: Arc<RwLock<usize>>,
    /// Wall-clock time spent with the pause menu open, subtracted from elapsed time when animating
    /// ghosts, so they freeze along with players while the menu is open.
    pub paused_accumulated: Arc<RwLock<Duration>>,
    pub body_rect_seen: Arc<RwLock<Option<Rect>>>,
    pub persistence: Arc<RwLock<Option<Arc<persistence::Persistence>>>>,
    pub persistence_writes: Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<persistence::PersistenceWrite>>>>,
    pub identity_uuid: uuid::Uuid,
    pub identity: String,
    start: Instant,
    pub runtime: Arc<tokio::runtime::Runtime>,
    /// Toggled by pressing `B` (see `draw_frame`'s `KeyboardEvent` handler). `scene::
    /// spawn_scene_window_on_toggle` opens or closes the on-demand Bevy scene window to match.
    pub bevy_scene_open: Arc<RwLock<bool>>,
    fps: Arc<RwLock<FpsCounter>>,
}

/// A plain frames-per-second sanity gauge for the terminal draw loop, per the user's own "seems a
/// bit perf poor" report. Counts frames over a rolling ~1s window rather than reporting instant
/// per-frame timing, since a single frame's delta is too noisy to read at a glance.
struct FpsCounter {
    window_start: Instant,
    frames_in_window: u32,
    last_fps: f32,
}

impl FpsCounter {
    fn new() -> Self {
        FpsCounter { window_start: Instant::now(), frames_in_window: 0, last_fps: 0.0 }
    }

    /// Call once per drawn frame. Returns the most recently completed window's fps, updated in
    /// place roughly once a second rather than every call.
    fn tick(&mut self) -> f32 {
        self.frames_in_window += 1;
        let elapsed = self.window_start.elapsed();
        if elapsed >= Duration::from_secs(1) {
            self.last_fps = self.frames_in_window as f32 / elapsed.as_secs_f32();
            self.frames_in_window = 0;
            self.window_start = Instant::now();
        }
        self.last_fps
    }
}

impl GameState {
    fn new(runtime: Arc<tokio::runtime::Runtime>, identity: String, identity_uuid: uuid::Uuid) -> Self {
        GameState {
            mario: Arc::new(RwLock::new(Vec::new())),
            visible_gamepad_candidates: Arc::new(RwLock::new(Vec::new())),
            gamepad_owned_by_me: Arc::new(RwLock::new(HashSet::new())),
            connected_players: Arc::new(RwLock::new(Vec::new())),
            local_mario_snapshot: Arc::new(RwLock::new(Vec::new())),
            remote_mario: Arc::new(RwLock::new(std::collections::HashMap::new())),
            outgoing_combat_events: Arc::new(RwLock::new(Vec::new())),
            incoming_combat_events: Arc::new(RwLock::new(Vec::new())),
            mario_wrap_cache: Arc::new(RwLock::new(None)),
            ghosts: Arc::new(RwLock::new(Vec::new())),
            cheat_menu_open: Arc::new(RwLock::new(false)),
            cheat_menu_selected: Arc::new(RwLock::new(0)),
            paused_accumulated: Arc::new(RwLock::new(Duration::ZERO)),
            body_rect_seen: Arc::new(RwLock::new(None)),
            persistence: Arc::new(RwLock::new(None)),
            persistence_writes: Arc::new(RwLock::new(None)),
            identity_uuid,
            identity,
            start: Instant::now(),
            runtime,
            bevy_scene_open: Arc::new(RwLock::new(false)),
            fps: Arc::new(RwLock::new(FpsCounter::new())),
        }
    }
}

/// Hosts the whole game inside a Bevy app, the same way `escher-bevy`'s generic terminal plugin
/// would, but calling this example's own draw function instead of a placeholder UI.
struct MarioTerminalPlugin;

impl Plugin for MarioTerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (terminal_startup, physics::spawn_platform, sfx::setup_mario_sfx));
        app.add_systems(PreUpdate, terminal_draw);
        app.add_systems(Update, (physics::update_mario_physics, scene::spawn_scene_window_on_toggle, scene::sync_scene_sprites).chain());
        app.add_systems(Last, terminal_exit);
    }
}

#[derive(Resource)]
struct TerminalHandle {
    surface: TerminalSurface<CrosstermBackend<Stdout>>,
    #[cfg(unix)]
    signal_flag: Arc<std::sync::atomic::AtomicUsize>,
}

fn terminal_startup(mut commands: Commands, event_loop_proxy: Res<bevy::winit::EventLoopProxyWrapper>) {
    let mut surface = TerminalSurface::<CrosstermBackend<Stdout>>::try_default().expect("failed to construct the terminal surface");

    crossterm::terminal::enable_raw_mode().expect("failed to enable raw mode");
    crossterm::execute!(surface.backend_mut(), crossterm::terminal::EnterAlternateScreen, crossterm::cursor::EnableBlinking)
        .expect("failed to prepare the terminal");

    #[cfg(unix)]
    let signal_flag = escher_bevy::terminal::spawn_signal_watcher(event_loop_proxy.clone());
    spawn_input_watcher(event_loop_proxy.clone());

    commands.insert_resource(TerminalHandle { surface, #[cfg(unix)] signal_flag });
}

fn terminal_draw(
    mut terminal: ResMut<TerminalHandle>,
    state: Res<GameState>,
    mut exit_evt: bevy::ecs::message::MessageWriter<AppExit>,
    colliders: bevy::ecs::system::Query<&physics::MarioCollider>,
) {
    #[cfg(unix)]
    if terminal.signal_flag.load(std::sync::atomic::Ordering::Relaxed) != 0 {
        exit_evt.write(AppExit::Success);
        return;
    }

    let colliders: Vec<(f32, f32, f32, f32)> = colliders.iter().map(|collider| collider.rect).collect();

    match draw_frame(&mut terminal.surface, &state, &colliders) {
        Ok(TerminalAction::Exit(_)) => {
            exit_evt.write(AppExit::Success);
        }
        Ok(_) => {}
        Err(error) => tracing::warn!("Terminal draw failed: {error}"),
    }
}

fn terminal_exit(terminal: Option<ResMut<TerminalHandle>>, mut exit_evt: MessageReader<AppExit>) {
    let Some(mut terminal) = terminal else { return };

    for _ in exit_evt.read() {
        let _ = crossterm::execute!(
            terminal.surface.backend_mut(),
            crossterm::style::ResetColor,
            crossterm::terminal::LeaveAlternateScreen,
        );
        if let Err(error) = crossterm::terminal::disable_raw_mode() {
            eprintln!("Failed to disable raw mode: {error}");
        }
        let _ = std::io::stdout().flush();

        #[cfg(unix)]
        escher_bevy::terminal::reraise_signal(&terminal.signal_flag);
    }
}

fn scoreboard_text(state: &GameState) -> String {
    let local_mario = state.mario.read();
    let connected_players = state.connected_players.read();
    if connected_players.is_empty() && local_mario.is_empty() {
        return "Escher Mario example, no gamepads connected yet".to_string();
    }

    let mut parts = Vec::new();
    for (candidate_id, name, _) in connected_players.iter() {
        let kills = local_mario.iter().find(|(_, id, _)| id == candidate_id).map(|(_, _, mario)| mario.kills);
        match kills {
            Some(kills) => parts.push(format!("{name}: {kills} kills")),
            None => parts.push(format!("{name}: (remote)")),
        }
    }
    parts.join("  |  ")
}

fn draw_frame(
    surface: &mut TerminalSurface<CrosstermBackend<Stdout>>,
    state: &GameState,
    colliders: &[(f32, f32, f32, f32)],
) -> Result<TerminalAction> {
    let area = surface.size()?;
    let body_width = area.width;
    let body_height = area.height.saturating_sub(HEADER_HEIGHT);
    *state.body_rect_seen.write() = Some(Rect { x: 0, y: HEADER_HEIGHT, width: body_width, height: body_height });

    let rows = {
        let mut cache = state.mario_wrap_cache.write();
        match cache.as_ref() {
            Some((width, rows)) if *width == body_width => rows.clone(),
            _ => {
                let rows = Arc::new(render::wrap_to_columns(BACKDROP_TEXT, body_width as usize));
                *cache = Some((body_width, rows.clone()));
                rows
            }
        }
    };

    let local_mario = state.mario.read();
    let connected_players = state.connected_players.read();
    let color_for = |candidate_id: &str, index: usize| -> (u8, u8, u8) {
        connected_players.iter().find(|(id, ..)| id == candidate_id).map(|(_, _, color)| *color).unwrap_or_else(|| physics::mario_player_color(index))
    };

    let mut sprites: Vec<(MarioState, (u8, u8, u8), usize)> =
        local_mario.iter().enumerate().map(|(index, (_, candidate_id, mario))| (*mario, color_for(candidate_id, index), index)).collect();

    for (candidate_id, packet) in state.remote_mario.read().iter() {
        if local_mario.iter().any(|(_, local_id, _)| local_id == candidate_id) {
            continue;
        }
        let index = sprites.len();
        let remote = MarioState {
            x: packet.x,
            y: packet.y,
            prev_x: packet.x,
            prev_y: packet.y,
            vx: packet.vx,
            vy: packet.vy,
            grounded: packet.grounded,
            jumps_used: packet.jumps_used,
            touching_wall: None,
            dust_effect: None,
            facing: 1.0,
            attack_cooldown: 0.0,
            // Combat events (see `relay::CombatEvent`) are synced now, but only applied to a
            // player's own owning instance (`physics::apply_local_death`), not reflected back into
            // how a third instance renders someone else's death here — a remote player still always
            // renders as idle and alive on a screen that isn't theirs, even mid-swing or dead.
            time_since_last_attack: physics::MARIO_ATTACK_SPAM_WINDOW + 1.0,
            attack_spam_stacks: 0,
            kills: 0,
            attack_effect: None,
            alive: true,
            lives: physics::MARIO_STARTING_LIVES,
            respawn_remaining: None,
            death_effect: None,
        };
        sprites.push((remote, color_for(candidate_id, index), index));
    }
    drop(local_mario);
    drop(connected_players);

    let elapsed_seconds = state.start.elapsed().saturating_sub(*state.paused_accumulated.read()).as_secs_f32();
    let ghost_positions: Vec<(f32, f32, (u8, u8, u8), f64)> = state
        .ghosts
        .read()
        .iter()
        .rev()
        .take(ghosts::MARIO_GHOST_RENDER_LIMIT)
        .map(|ghost| {
            let (x, y) = ghosts::mario_ghost_position(elapsed_seconds, ghost.drift);
            let flicker = ghosts::mario_ghost_flicker(elapsed_seconds, ghost.drift.flicker_phase);
            (x, y, ghost.color, flicker)
        })
        .collect();

    let menu_lines = if *state.cheat_menu_open.read() { Some(render::cheat_menu_lines(*state.cheat_menu_selected.read())) } else { None };
    let body_content = render::mario_body_text(&rows, &sprites, &ghost_positions, menu_lines.as_deref(), body_width, body_height, colliders);
    let fps = state.fps.write().tick();
    let header_text = format!("{}  |  {fps:.0} fps", scoreboard_text(state));
    let bevy_scene_open = state.bevy_scene_open.clone();

    surface.draw_with_poll_timeout(
        move |root| {
            root.style(FlexDirection::Column)
                // `B` opens the same running game as a real Bevy scene window alongside the
                // terminal one (see `scene::spawn_scene_window_on_toggle`). Pressing it again
                // closes that window back down.
                .handle::<KeyboardEvent>(move |event| {
                    if event.code == Code::KeyB && event.state == KeyState::Down {
                        let mut open = bevy_scene_open.write();
                        *open = !*open;
                    }
                })
                .slot::<Header>(move |header| header.style(Size::height(HEADER_HEIGHT)).content(Some(header_text)))
                .slot::<Body>(move |body| body.content(Some(body_content)))
        },
        Duration::ZERO,
    )
}
