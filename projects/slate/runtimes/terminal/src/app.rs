use core::time::Duration;

// TODO: Restrict this to the std feature and find a
// fallback for wasm, embedded, ffi, etc.
use std::io::Stdout;

use eyre::Error;

use ratatui::prelude::Backend;
use ratatui::prelude::CrosstermBackend;

use crate::surface::TerminalSurface;

#[derive(Debug)]
pub struct TerminalApp<B: Backend = CrosstermBackend<Stdout>> {
    surface: Option<TerminalSurface<B>>,
    tick_speed: Duration,
}

impl<B: Backend> TerminalApp<B> {
    pub fn new() -> Self {
        TerminalApp::<B> {
            surface: None,
            tick_speed: Duration::from_millis(16),
        }
    }
    
    pub fn with_surface(mut self, surface: TerminalSurface<B>) -> Self {
        self.surface = Some(surface);
        self // ..
    }
    
    pub fn with_tick_speed(mut self, speed: Duration) -> Self {
        self.tick_speed = speed;
        self // ..
    }
}

impl TerminalApp<CrosstermBackend<Stdout>> {
    pub fn run<F>(self, draw_surface_fn: F) -> Result<(), Error>
    where
        F: Fn(&mut TerminalSurface<CrosstermBackend<Stdout>>) -> Result<TerminalAction, Error>,
    {
        if let Some(mut surface) = self.surface {
            let mut stdout = surface.stdout();
            
            crossterm::execute!(
                &mut stdout,
                crossterm::event::EnableFocusChange,
                crossterm::event::EnableMouseCapture,
                crossterm::event::EnableBracketedPaste,
                crossterm::cursor::EnableBlinking,
            )?;
            
            surface.clear()?;
            
            loop {
                match draw_surface_fn(&mut surface) {
                    Ok(action) => match action {
                        #[allow(unused)]
                        TerminalAction::Exit(code) => {
                            #[cfg(feature="dev")]
                            tracing::debug!("Exiting with code '{:}' ..", code);
                            break; // <3
                        }
                        TerminalAction::NoOp => {
                            #[cfg(all(feature="dev", feature="verbose"))]
                            tracing::trace!("No-op ..");
                            continue;
                        }
                    }
                    Err(error) => {
                        tracing::error!("Failed to draw TerminalApp surface: {:}", error);
                        break; // </3
                    }
                }
            }
            
            surface.set_cursor_position((0, 0))?;
            
            surface.clear()?;
            
            crossterm::execute!(
                &mut stdout,
                crossterm::event::DisableFocusChange,
                crossterm::event::DisableMouseCapture,
                crossterm::event::DisableBracketedPaste,
                crossterm::cursor::DisableBlinking,
            )?;
            
            ratatui::restore();
        };
        
        Ok(())
    }
}

#[derive(Default, Debug)]
pub enum TerminalAction {
    Exit(i8),
    #[default]
    NoOp,
}
