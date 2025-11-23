#![allow(unused)]

use std::any::TypeId;
use std::fmt::Write;
use std::io::Error as IoError;
use std::io::Stderr;
use std::io::Stdin;
use std::io::Stdout;
use std::marker::PhantomData;
use std::time::Duration;

use derive_more::*;

use bumpalo_herd::Herd;

use color_eyre::Result;

use num_traits::AsPrimitive;

use ansi_to_tui::IntoText;

use num_traits::Zero;

use ratatui::crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::style::Color as RatatuiColor;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Padding;
use ratatui::widgets::Borders;
use ratatui::widgets::BorderType;
use ratatui::widgets::Paragraph;
use ratatui::crossterm::event::Event as CrosstermEvent;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::crossterm::event::KeyEventState;

use slate_core::event::keyboard::Code;
use slate_core::event::keyboard::Key;
use slate_core::event::keyboard::KeyState;
use slate_core::event::keyboard::KeyboardEvent;
use slate_core::event::keyboard::Location;
use slate_core::event::keyboard::Modifiers;
use slate_core::event::keyboard::NamedKey;
use slate_core::surface::Surface;
use slate_core::scaffold::Scaffold;
use slate_core::style::Unit;
use slate_core::style::Value;
use slate_core::style::Property;
use slate_core::style::Size;
use slate_core::style::FlexDirection;
use slate_core::style::Edge;
use slate_core::style::Gap;
use slate_core::style::BackgroundColor;
use slate_core::style::ContentColor;
use slate_core::style::FontStyle;
use slate_core::style::Border;
use slate_core::content::LineCounter;

use crate::app::TerminalAction;

//---
#[derive(Deref, DerefMut)]
pub struct TerminalSurface<B: Backend> {
    #[deref]
    #[deref_mut]
    terminal: Terminal<B>,
    allocator: Herd,
}

impl<B: Backend + core::fmt::Debug> core::fmt::Debug for TerminalSurface<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RatatuiSurface").field("terminal", &self.terminal).finish()
    }
}

impl<B: Backend> TerminalSurface<B> {
    pub fn new(backend: B) -> Result<Self> {
        Ok(TerminalSurface {
            terminal: Terminal::<B>::new(backend)?,
            allocator: Herd::new(),
        })
    }
}

impl TerminalSurface<CrosstermBackend<Stdout>> {
    pub fn try_default() -> Result<Self> {
        Ok(TerminalSurface {
            terminal: Terminal::new(CrosstermBackend::new(std::io::stdout()))?,
            allocator: Herd::new(),
        })
    }
}

impl TerminalSurface<CrosstermBackend<Stdout>> {
    pub fn stdout(&self) -> Stdout {
        std::io::stdout()
    }

    pub fn stderr(&self) -> Stderr {
        std::io::stderr()
    }

    pub fn stdin(&self) -> Stdin {
        std::io::stdin()
    }
}

impl Surface for TerminalSurface<CrosstermBackend<Stdout>> {
    type Event = TerminalAction;
    
    fn draw<F>(&mut self, draw_scaffold_fn: F) -> Result<TerminalAction>
    where
        F: for<'ctx> FnOnce(Scaffold<'ctx>) -> Scaffold<'ctx>,
    {
        let arena = self.allocator.get();

        let mut scaffold = draw_scaffold_fn(Scaffold::new_in(arena.as_bump()));
        
        // TODO: Walk the scaffold and apply optimizations:
        //  - Unpack event handlers (where possible).
        //  - Build/validate/normalize styles.
        //  - Save a snapshot of each item.
        // 
        // TODO: Optionally (via cfg) apply retained-mode rules:
        //  - Find and apply Node ids (lookup based on index + hash).
        //  - Nodes with changes should be marked.

        match self.terminal.draw(|frame| Self::render(&scaffold, frame, &frame.area()))? {
            // _frame if true => {
            //     // TODO: Send out post-render events to the runtime.
            //     // TODO: Save a snapshot of the completed_frame.
            // }
            completed_frame => {
                #[cfg(all(feature="dev", feature="verbose"))]
                tracing::tracing!("Frame Area: {:?}", completed_frame.area);
                
                if crossterm::event::poll(Duration::from_secs(1))? {
                    let event = ratatui::crossterm::event::read()?;
                    return self.dispatch(&mut scaffold, &event);
                }
            }
        }
        
        Ok(TerminalAction::NoOp)
    }
}

impl TerminalSurface<CrosstermBackend<Stdout>> {
    fn render(scaffold: &Scaffold, frame: &mut Frame, frame_bounds: &Rect) {
        if !scaffold.is_enabled() {
            return;
        }
        
        let mut content_bounds = *frame_bounds;
        let mut border_size = (0u16, 0u16);

        if scaffold.is_debug() {
            // TODO: Draw debug information on this!
            let debug_block = Block::default()
                .title_top({
                    let Rect { x, y, .. } = content_bounds;
                    Line::from(format!("Pos: {}x{}", x, y))
                        .alignment(Alignment::Left)
                })
                .title_bottom({
                    let Rect { width, height, .. } = content_bounds;
                    Line::from(format!("Size: {}x{}", width, height))
                        .alignment(Alignment::Right)
                })
                .border_style(Style::new().dim())
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL);
            
            let xray_inner_area = debug_block.inner(content_bounds);
            frame.render_widget(debug_block, *frame_bounds);
            
            content_bounds = xray_inner_area;
            
            border_size.0 += 1;
            border_size.1 += 1;
        }

        if let Some(borders) = scaffold.get_styles().get(&TypeId::of::<Border>()) {
            for border in borders.iter() {
                if let Property::Border(Border(border_edge, border_value, border_style, border_color)) = border && !border_value.is_zero() {
                    let border_block = Block::default()
                        // .title_top(Line::from("TODO: Border labels ..").alignment(Alignment::Left))
                        // TODO: Use the border_weight to determine 
                        .border_style(Style::new())
                        .border_type(BorderType::Rounded)
                        .borders(Borders::ALL);
                    let border_inner_area = border_block.inner(content_bounds);
                    frame.render_widget(border_block, content_bounds);
                    content_bounds = border_inner_area;
                    border_size.0 += 1;
                    border_size.1 += 1;
                }
            }
        }

        //--
        let flex_direction = scaffold
            .get_styles()
            .iter()
            .find(|(type_id, values)| **type_id == TypeId::of::<FlexDirection>())
            .map(|(_, values)| match values.first() {
                Some(Property::FlexDirection(direction)) => *direction,
                _ => FlexDirection::Column,
            })
            .unwrap_or_default();

        let mut layout_constraints = Vec::with_capacity(scaffold.get_slots().len() + 1);
        
        if let Some(content) = scaffold.get_content() {
            let mut lines = LineCounter::new();
            
            if let Err(error) = write!(&mut lines, "{}", content) {
                tracing::error!("Failed to count content lines: {}", error)
            }

            layout_constraints.push(match flex_direction {
                FlexDirection::Column => Constraint::Length(lines.count()),
                FlexDirection::Row => Constraint::Fill(1),
            });
        }

        //--
        for (_, slot) in scaffold.get_slots().iter() {
            if slot.is_enabled() {
                layout_constraints.push({
                    slot.get_styles()
                        .get(&TypeId::of::<Size>())
                        .and_then(|values| {
                            values
                                .iter()
                                .find(|value| matches!(value, Property::Size(..)))
                                .and_then(|value| match value {
                                    Property::Size(Size(Value::Px(width), ..)) if flex_direction.is_row() => {
                                        Some(Constraint::Length(<Unit as AsPrimitive<u16>>::as_(*width)))
                                    }
                                    Property::Size(Size(_, Value::Px(height), ..)) if flex_direction.is_column() => {
                                        Some(Constraint::Length(<Unit as AsPrimitive<u16>>::as_(*height)))
                                    }
                                    _ => None
                                })
                        })
                        .unwrap_or(Constraint::Min(0))
                });
            }
        }
        
        //---
        let mut layout = Layout::new(into_direction(flex_direction), layout_constraints);
        
        for property in scaffold.get_styles().iter().flat_map(|style| style.1) {
            match property {
                Property::Margin(margin) => {
                    layout = layout
                        .vertical_margin(margin.1.as_())
                        .horizontal_margin(margin.1.as_())
                }
                Property::Padding(padding) => {
                    //..
                }
                Property::Gap(Gap(gap)) => {
                    layout = layout.spacing::<u16>(gap.as_())
                }
                _ => continue
            }
        }
        
        //---
        let layout_areas = layout.split(content_bounds);
        let mut slot_areas = layout_areas.into_iter();
        let content_area = scaffold.get_content().and_then(|_| slot_areas.next());

        if let Some(content_area) = content_area
        && let Some(content) = scaffold.get_content() {
            // TODO: Use the content area calculated above.
            let mut block = Block::default();
            let mut style = Style::default();
            let mut text = Text::default();
            
            for property in scaffold.get_styles().iter().flat_map(|style| style.1) {
                match property {
                    Property::Margin(margin) => {
                        tracing::warn!("Margin not yet implemented ..");
                    }
                    Property::Padding(padding) => {
                        // TODO: Add to the previously assigned padding rules ..
                        block = block.padding(into_padding(padding));
                    }
                    Property::FontStyle(font_style) => {
                        style = match font_style {
                            FontStyle::Normal => style.not_italic(),
                            FontStyle::Italic => style.italic(),
                        }
                    }
                    Property::BackgroundColor(color) => {
                        style = style.bg({
                            color
                                .map(|color| RatatuiColor::Rgb(color.red, color.green, color.blue))
                                .unwrap_or_default()
                        });
                    }
                    Property::ContentColor(color) => {
                        style = style.fg({
                            color
                                .map(|color| RatatuiColor::Rgb(color.red, color.green, color.blue))
                                .unwrap_or_default()
                        });
                    }
                    _ => continue
                }
            }
            
            // let pos = content_area.as_position();
            // let size = content_area.as_size();
            let widget = match content.to_string().into_text() {
                Ok(text) => Paragraph::new(text).style(style),
                Err(error) => {
                    tracing::error!("Failed to get rich text from content: {}", error);
                    Paragraph::new("ERROR".red())
                }
            };

            frame.render_widget(widget, *content_area);
        }
        
        for (i, slot_area) in slot_areas.enumerate() {
            match scaffold.get_slots().values().nth(i) {
                Some(slot) => Self::render(slot, frame, slot_area),
                None => tracing::warn!("Failed to get slot {}!", i),
            }
        }
    }
    
    fn dispatch(&self, scaffold: &mut Scaffold, event: &CrosstermEvent) -> Result<TerminalAction> {
        match event {
            CrosstermEvent::FocusGained => {
                // tracing::debug!("TODO: Handle events!");
            }
            CrosstermEvent::FocusLost => {
                // tracing::debug!("TODO: Handle events!");
            }
            CrosstermEvent::Key(event) => {
                let event = unpack_keyboard_event(event);
                
                // TODO: This should be optional ..
                if event.code == Code::Escape && event.state == KeyState::Up {
                    return Ok(TerminalAction::Exit(0));
                }
                
                scaffold.dispatch::<KeyboardEvent>(&event);
            }
            CrosstermEvent::Mouse(mouse_event) => {
                // tracing::debug!("TODO: Handle events!");
            }
            CrosstermEvent::Paste(_) => {
                // tracing::debug!("TODO: Handle events!");
            }
            CrosstermEvent::Resize(_, _) => {
                // tracing::debug!("TODO: Handle events!");
            }
            unhandled_event => {
                #[cfg(all(feature="dev", feature="verbose"))]
                tracing::warn!("Surface Event: {:?}", unhandled_event);
            }
        }
        
        scaffold.dispatch::<CrosstermEvent>(event)?;
        
        Ok(TerminalAction::NoOp)
    }
}

impl<B: Backend> TerminalSurface<B> {
    pub fn clear(&mut self) -> Result<(), IoError> {
        self.terminal.clear()
    }
}

//---
pub fn into_margin(margin: &slate_core::style::Margin) -> ratatui::widgets::block::Padding {
    match margin.1 {
        Value::Px(px) => match margin.0 {
            Edge::All => ratatui::widgets::block::Padding::uniform(px.as_()),
            Edge::Top => ratatui::widgets::block::Padding::top(px.as_()),
            Edge::Right => ratatui::widgets::block::Padding::right(px.as_()),
            Edge::Bottom => ratatui::widgets::block::Padding::bottom(px.as_()),
            Edge::Left => ratatui::widgets::block::Padding::left(px.as_()),
            Edge::None => ratatui::widgets::block::Padding::ZERO,
        },
        unhandled_value => {
            // TODO: Document this behavior in Known Issues.
            tracing::warn!("Margin not yet implemented for value {:?}; using default ..", unhandled_value);
            ratatui::widgets::block::Padding::default()
        }
    }
}

pub fn into_padding(padding: &slate_core::style::Padding) -> ratatui::widgets::block::Padding {
    match padding.1 {
        Value::Px(px) => match padding.0 {
            Edge::All => ratatui::widgets::block::Padding::uniform(px.as_()),
            Edge::Top => ratatui::widgets::block::Padding::top(px.as_()),
            Edge::Right => ratatui::widgets::block::Padding::right(px.as_()),
            Edge::Bottom => ratatui::widgets::block::Padding::bottom(px.as_()),
            Edge::Left => ratatui::widgets::block::Padding::left(px.as_()),
            Edge::None => ratatui::widgets::block::Padding::ZERO,
        },
        unhandled_value => {
            tracing::warn!("Padding not yet implemented for value {:?}; using default ..", unhandled_value);
            ratatui::widgets::block::Padding::default()
        }
    }
}

pub fn into_direction(flex_direction: FlexDirection) -> ratatui::layout::Direction {
    match flex_direction {
        FlexDirection::Column => Direction::Vertical,
        FlexDirection::Row => Direction::Horizontal,
    }
}

pub fn into_constraint(size: &Size, direction: FlexDirection, area: &Rect) -> Constraint {
    match size {
        Size(Value::Px(width), _, _) => Constraint::Length(width.as_()),
        Size(_, Value::Px(height), _) => Constraint::Length(height.as_()),
        Size(_, _, _) => match direction {
            FlexDirection::Row => Constraint::Length(area.width),
            FlexDirection::Column => Constraint::Length(area.height),
        },
    }
}

fn unpack_keyboard_event(event: &KeyEvent) -> KeyboardEvent {
    KeyboardEvent {
        state: unpack_state(&event.kind),
        key: unpack_key(&event.code),
        code: unpack_code(&event.code),
        location: unpack_location(&event.state),
        modifiers: unpack_modifiers(&event.modifiers),
        repeat: matches!(&event.kind, KeyEventKind::Repeat),
        is_composing: false, // ??
    }
}

fn unpack_modifiers(modifiers: &KeyModifiers) -> Modifiers {
    match modifiers {
        &KeyModifiers::ALT => Modifiers::ALT,
        &KeyModifiers::SHIFT => Modifiers::SHIFT,
        &KeyModifiers::CONTROL => Modifiers::CONTROL,
        &KeyModifiers::SUPER => Modifiers::META,
        &KeyModifiers::HYPER => Modifiers::META,
        &KeyModifiers::META => Modifiers::META,
        #[allow(unused)]
        unhandled_modifier => {
            #[cfg(all(feature="dev", feature="verbose"))]
            tracing::warn!("Unhandled Key Modifier: {:?}", unhandled_modifier);
            Modifiers::empty()
        }
    }
}

fn unpack_state(code: &KeyEventKind) -> KeyState {
    match code {
        KeyEventKind::Press => KeyState::Down,
        KeyEventKind::Repeat => KeyState::Down,
        KeyEventKind::Release => KeyState::Up,
    }
}

fn unpack_key(code: &KeyCode) -> Key {
    match code {
        KeyCode::Esc => Key::Named(NamedKey::Escape),
        KeyCode::Backspace => Key::Named(NamedKey::Backspace),
        #[allow(unused)]
        unhandled_key => {
            #[cfg(all(feature="dev", feature="verbose"))]
            tracing::warn!("Unhandled Key: {:?}", unhandled_key);
            Key::default()
        }
    }
}

fn unpack_code(code: &KeyCode) -> Code {
    match code {
        KeyCode::Esc => Code::Escape,
        KeyCode::Backspace => Code::Backspace,
        #[allow(unused)]
        unhandled_code => {
            #[cfg(all(feature="dev", feature="verbose"))]
            tracing::warn!("Unhandled Key: {:?}", unhandled_code);
            Code::default()
        }
    }
}

fn unpack_location(state: &KeyEventState) -> Location {
    Location::Standard
}
