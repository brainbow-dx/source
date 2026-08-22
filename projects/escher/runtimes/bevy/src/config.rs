use bevy::color::Color as BevyColor;
use bevy::window::ExitCondition;

pub struct EscherBevyConfig {
    pub bevy_defaults: bool,
    pub log_filter: String,
    pub asset_dir: String,
    pub clear_color: BevyColor,
    /// Whether the primary window is shown as soon as it's created. Most examples launch a
    /// process specifically to show a window, so the default is `true`; a caller that only wants
    /// the window to appear once something later decides it's actually needed (e.g. `apps/anvil`,
    /// whose companion webview window has nothing to show until a `/scene` command arrives) sets
    /// this `false` and reveals the window itself when that moment comes.
    pub window_visible: bool,
    /// Whether closing the window ends the process. Most examples launch a process to show
    /// exactly that window, so the default (`ExitCondition::OnAllClosed`) is right for them; a
    /// caller whose process does other real work independent of any window (e.g. `apps/anvil`,
    /// whose terminal UI has to keep running after its companion webview window is closed — a
    /// closed window there just means "no scene open right now," not "quit") wants
    /// `ExitCondition::DontExit` instead, and has to send its own `AppExit` for whatever *should*
    /// end the process (there, a hard exit from the terminal side).
    pub exit_condition: ExitCondition,
    /// Whether clicking the window's own close button despawns it (Bevy's default: the window,
    /// its webview if any, and any native state attached to it are all gone, and — combined with
    /// `exit_condition: OnAllClosed`, also the default — the whole process exits). A caller that
    /// wants the window to survive its own close button (e.g. `apps/anvil`: closing the companion
    /// webview window should just hide it, ready to be shown again, not tear down and rebuild the
    /// whole native webview/chrome-bar stack on every open) sets this `false` and handles
    /// `WindowCloseRequested` itself — see that type's own doc comment for the caveats of doing
    /// so (most notably: nothing else will close the window for you anymore).
    pub close_when_requested: bool,
    /// The primary window's title-bar text, and its initial size in logical points. Whatever
    /// opens the window is the one that knows what it's called and how big it should start —
    /// this used to be hardcoded to `"Escher"`/800x600 inside `EscherBevyPlugin` itself, which
    /// meant every app built on it got the same generic title no matter its own identity.
    pub window_title: String,
    pub window_width: f32,
    pub window_height: f32,
    /// Whether `WindowPlugin` creates a primary window at all. Most examples want one
    /// immediately (`true`, the default); a caller whose windows are all opened on demand later
    /// (e.g. `apps/anvil`: every scene window is spawned fresh in response to `/scene`, there's
    /// no single window to pre-create at startup anymore) sets this `false` and starts headless.
    pub spawn_primary_window: bool,
    /// Whether the primary window is hidden from the taskbar/Alt-Tab (Windows-only, per
    /// `bevy::window::Window::skip_taskbar`'s own doc comment). Most windows want to show up
    /// normally (`false`, the default); a caller spawning a window purely to hold OS input focus
    /// rather than to be looked at (e.g. `examples/mario`'s Windows-only focus-holder window, see
    /// its own doc comment) sets this `true` so it doesn't clutter the taskbar with an invisible
    /// entry the user can't do anything useful with.
    pub skip_taskbar: bool,
    /// Whether `EscherBevyPlugin` registers its own `terminal::TerminalPlugin` (behind the
    /// `terminal` Cargo feature). Most examples that enable the feature want this crate's generic
    /// `Scaffold`-drawn terminal UI too, so the default is `true`; a caller that only needs the
    /// feature's plain helper functions (`spawn_input_watcher`/`spawn_signal_watcher`/
    /// `reraise_signal`) because it draws its *own* terminal UI (e.g. `apps/anvil`'s
    /// `AssistantTerminalPlugin`) must set this `false` — running both at once means two
    /// independent pollers/drawers racing for the same OS terminal, exactly the hazard
    /// `TerminalSurface::draw`'s own doc comment warns about: the two plugins' competing
    /// `Scaffold` draws interleave character-by-character into the same alternate-screen buffer,
    /// corrupting the header and doubling per-frame input polling/redraw work.
    pub spawn_terminal_plugin: bool,
}

impl Default for EscherBevyConfig {
    fn default() -> Self {
        EscherBevyConfig {
            bevy_defaults: true,
            log_filter: String::default(),
            asset_dir: String::default(),
            clear_color: BevyColor::hsla(0., 0., 0., 0.),
            window_visible: true,
            exit_condition: ExitCondition::OnAllClosed,
            close_when_requested: true,
            window_title: String::from("Escher"),
            window_width: 800.0,
            window_height: 600.0,
            spawn_primary_window: true,
            skip_taskbar: false,
            spawn_terminal_plugin: true,
        }
    }
}

impl EscherBevyConfig {
    pub fn with_bevy_defaults(mut self, bevy_defaults: bool) -> Self {
        self.bevy_defaults = bevy_defaults;
        self
    }

    pub fn with_log_filter<S: Into<String>>(mut self, log_filter: S) -> Self {
        self.log_filter = log_filter.into();
        self
    }

    pub fn with_asset_dir<S: Into<String>>(mut self, asset_dir: S) -> Self {
        self.asset_dir = asset_dir.into();
        self
    }

    pub fn with_clear_color(mut self, color: BevyColor) -> Self {
        self.clear_color = color;
        self
    }

    pub fn with_window_visible(mut self, window_visible: bool) -> Self {
        self.window_visible = window_visible;
        self
    }

    pub fn with_exit_condition(mut self, exit_condition: ExitCondition) -> Self {
        self.exit_condition = exit_condition;
        self
    }

    pub fn with_close_when_requested(mut self, close_when_requested: bool) -> Self {
        self.close_when_requested = close_when_requested;
        self
    }

    pub fn with_window_title<S: Into<String>>(mut self, window_title: S) -> Self {
        self.window_title = window_title.into();
        self
    }

    pub fn with_window_size(mut self, width: f32, height: f32) -> Self {
        self.window_width = width;
        self.window_height = height;
        self
    }

    pub fn with_spawn_primary_window(mut self, spawn_primary_window: bool) -> Self {
        self.spawn_primary_window = spawn_primary_window;
        self
    }

    pub fn with_skip_taskbar(mut self, skip_taskbar: bool) -> Self {
        self.skip_taskbar = skip_taskbar;
        self
    }

    pub fn with_spawn_terminal_plugin(mut self, spawn_terminal_plugin: bool) -> Self {
        self.spawn_terminal_plugin = spawn_terminal_plugin;
        self
    }
}
