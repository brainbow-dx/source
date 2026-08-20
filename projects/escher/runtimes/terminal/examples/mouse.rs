//! Verification example for widget hit-testing and click-drag text selection —
//! `TerminalSurface`'s mouse support. A clickable box in the header toggles color and label on
//! click (proves `ClickEvent` handlers actually fire); the body is plain selectable text (proves
//! click-drag highlights and copies to the system clipboard).

use std::io::Stdout;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use color_eyre::Result;

use ratatui::backend::CrosstermBackend;

use escher_core::element::*;
use escher_core::style::*;
use escher_core::surface::Surface;

use escher_terminal::app::TerminalApp;
use escher_terminal::app::TerminalAction;
use escher_terminal::surface::ClickEvent;
use escher_terminal::surface::TerminalSurface;

static CLICKED: AtomicBool = AtomicBool::new(false);

fn main() -> Result<ExitCode> {
    color_eyre::install()?;

    tracing_subscriber::fmt().with_env_filter("info").with_target(false).without_time().init();

    TerminalApp::new()
        .with_surface(TerminalSurface::<CrosstermBackend<Stdout>>::try_default()?)
        .run(draw_mouse_demo)?;

    Ok(ExitCode::SUCCESS)
}

fn draw_mouse_demo(surface: &mut TerminalSurface<CrosstermBackend<Stdout>>) -> Result<TerminalAction> {
    surface.draw(move |root| {
        let clicked = CLICKED.load(Ordering::Relaxed);

        root.with_style(FlexDirection::Column)
            .with_slot::<Header>(move |header| {
                header
                    .with_style(Size::height(3))
                    .with_style(FlexDirection::Row)
                    .with_style(BackgroundColor::from(if clicked { "#668866ff" } else { "#886666ff" }))
                    .with_handler::<ClickEvent>(move |_event: &ClickEvent| {
                        CLICKED.store(!clicked, Ordering::Relaxed);
                    })
                    .with_content(Some(if clicked { "Clicked! Click again to toggle." } else { "Click me!" }))
            })
            .with_slot::<Body>(|body| {
                body.with_style(FlexDirection::Column).with_content(Some(
                    "Click-drag across this text to select it, then release to copy it to the \
                     system clipboard. Selected text renders in reverse video.",
                ))
            })
    })
}
