#![feature(allocator_api)]

extern crate alloc;

use std::process::ExitCode;
use std::io::Stdout;

use alloc::sync::Arc;
use alloc::fmt::Debug;

use color_eyre::owo_colors::OwoColorize;
use color_eyre::Result;

use clap::Parser;

use derive_more::*;

use parking_lot::RwLock;

use atlas::tracing::TracingSubscriber;
use atlas::store::tokio::LocalStore;

// TODO: Re-export from slate_terminal.
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::Event as CrosstermEvent;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::crossterm::event::KeyCode;

// use slate_core::draw::DrawReport;
use slate_core::surface::Surface;
use slate_core::element::*;
use slate_core::style::*;

use slate_terminal::app::TerminalAction;
use slate_terminal::app::TerminalApp;
use slate_terminal::surface::TerminalSurface;

//---
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    /// TODO
    #[arg(short, long, default_value="trace")]
    log_level: String,

    /// TODO
    #[arg(long, default_value="false")]
    console: bool,
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    
    color_eyre::install()?;
    
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .with_thread_names(false)
        .with_line_number(false)
        .with_target(false)
        .with_file(false)
        .with_ansi(true)
        .without_time()
        .init();

    //--
    let dashboard_state = DashboardState::new();
    
    tracing::subscriber::with_default(dashboard_state.tracing_stream.clone(), || {
        TerminalApp::new()
            .with_surface(TerminalSurface::<CrosstermBackend<Stdout>>::try_default()?)
            .run(|surface| draw_dashboard(surface, &dashboard_state))
    })?;
    
    //--
    tracing::info!("Bye! <3");
    Ok(ExitCode::SUCCESS)
}

fn draw_dashboard(
    surface: &mut TerminalSurface<CrosstermBackend<Stdout>>,
    state: &DashboardState,
) -> Result<TerminalAction> {
    surface.draw(move |terminal_root| {
        terminal_root
            // Keys can be variant between state entries ..
            .with_state("some-key-value", state.tracing_stream.clone())
            .with_state(0, state.tracing_stream.clone())
            .with_handler::<CrosstermEvent>({
                let user_input = state.user_input.clone();
                move |event| match event {
                    CrosstermEvent::Key(key) => match key.code {
                        KeyCode::Char(key_char) => {
                            if key.kind != KeyEventKind::Release {
                                user_input.write().push(key_char);
                            }
                        }
                        KeyCode::Backspace => {
                            if key.kind != KeyEventKind::Release {
                                user_input.write().pop();
                            }
                        }
                        KeyCode::Enter => {
                            if key.kind != KeyEventKind::Release {
                                let mut user_input = user_input.write();
                                let name = "@moodring.dev"; // TODO
                                let message = user_input.as_str();
                                
                                tracing::debug!("{} {}", name.cyan(), message);
                                user_input.clear();
                            }
                        }
                        _key_code => {
                            #[cfg(all(feature="dev", feature="verbose"))]
                            tracing::trace!("Unhandled key press: {}", _key_code);
                        }
                    }
                    _event => {
                        #[cfg(all(feature="dev", feature="verbose"))]
                        tracing::trace!("Unhandled crossterm event: {:?}", _event);
                    }
                }
            })
            .with_slot::<Header>(|header| {
                header
                    .with_debug(false)
                    .with_style(FlexDirection::Row)
                    // .with_style(Margin::all(1))
                    .with_style(Size::height(4))
                    .with_style(Border::new(1, BorderStyle::Solid, None))
                    .with_slot::<String>(|metadata| {
                        metadata
                            .with_slot::<Legend>(|legend| {
                                legend
                                    .with_content(Some("Slate Terminal Example"))
                            })
                            .with_content(Some("Tracing Stream++"))
                            .with_slot::<String>(|content| {
                                content
                                    .with_element(Text::<&str>::new("[Some Status Text]"))
                            })
                    })
                    .with_slot::<String>(|users| {
                        users
                            .with_style(slate_core::style::Size::width(16))
                            .with_content(Some("Tracing:"))
                            .with_slot::<Legend>(|names| {
                                names
                                    .with_style(Gap(1.into()))
                                    .with_style(FlexDirection::Row)
                                    .with_slot::<u8>(|active| {
                                        active
                                            .with_style(Size::width(1))
                                            .with_content(Some("1"))
                                    })
                                    .with_slot::<u16>(|available| {
                                        available
                                            .with_style(FontStyle::Italic)
                                            .with_style(ContentColor::from("#555"))
                                            .with_content(Some("of 8"))
                                    })
                            })
                    })
                    .with_slot::<String>(|bots| {
                        bots
                            .with_style(Size::width(16))
                            .with_content(Some("Perf:"))
                            .with_slot::<Legend>(|names| {
                                names
                                    .with_style(Gap(1.into()))
                                    .with_style(FlexDirection::Row)
                                    .with_slot::<u8>(|active| {
                                        active
                                            // TODO: Get width 
                                            .with_style(Size::width(4))
                                            .with_content(Some("59.4"))
                                    })
                                    .with_slot::<u16>(|available| {
                                        available
                                            // TODO: Use gap on the parent instead ..
                                            .with_style(FontStyle::Italic)
                                            .with_style(ContentColor::from("#555"))
                                            .with_content(Some("fps"))
                                    })
                            })
                    })
            })
            .with_slot::<Body>(|console| {
                // The tracing stream should be modified to take and hold 
                // tracing spans + debug information. When that happens,
                // we'll have to unpack this stream more deliberately.
                console
                    // TODO: Move to a dedicated tracing-stream crate 
                    //  with a "ui" feature for Slate types.
                    .with_style(ScrollPosition::new(0))
                    // .with_element(TracingStreamDisplay::from(&tracing_stream)).
                    .with_content(Some(state.tracing_stream_content(100, 100)))
            })
            .with_slot::<Footer>(|footer| {
                footer
                    // .with_debug(true)
                    .with_style(FlexDirection::Row)
                    .with_style(Size::height(1))
                    // .with_style(Border(Unit::from(1), None, Color(None)))
                    // TODO: Implement `From<Option<Arc<RwLock<String>>> for Input` ..
                    .with_element(Input::<String>::new(state.user_input.read().to_owned()))
            })
    })
}

// TODO: #[derive(State)]
#[derive(Debug, Clone, Copy)]
#[derive(PartialEq, Eq, Hash)]
pub enum GeneralStoreKey {
    UnhandledEvent,
    Error,
}

#[derive(Default, From, Clone)]
pub struct DashboardState {
    pub user_input: Arc<RwLock<String>>,
    pub tracing_stream: TracingSubscriber<String>,
    pub general_store: LocalStore<GeneralStoreKey, String>,
}

impl DashboardState {
    pub fn new() -> Self {
        DashboardState {
            user_input: Arc::default(),
            tracing_stream: TracingSubscriber::with_capacity(1000),
            general_store: LocalStore::<GeneralStoreKey, String>::default(),
        }
    }
}

impl DashboardState {
    // TODO: Move this to the TracingStream formatter itself.
    pub fn tracing_stream_content(&self, rows: usize, columns: usize) -> String {
        let capacity = rows * (columns + rows);
        let mut content = String::with_capacity(capacity);
        
        let tracing_stream = self.tracing_stream.entries().read();
        let lines = tracing_stream.iter().rev().take(rows).rev();
        
        for line in lines {
            content.push_str(line);
            content.push('\n');
        }
        
        if content.ends_with('\n') {
            content.pop();
        }
                
        content
    }
}
