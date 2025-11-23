#![feature(allocator_api)]
#![feature(unboxed_closures)]

#![allow(unused)]

extern crate alloc;

use alloc::fmt::Debug;

use color_eyre::owo_colors::OwoColorize;

use crossterm::event::KeyModifiers;
use crossterm::event::MouseButton;

use num_traits::AsPrimitive;

use parking_lot::RwLock;

use quick_xml::events::attributes::Attribute;
use quick_xml::name::QName;
use ratatui::style::Stylize;

use core::hash::Hash;

use std::borrow::Cow;
use std::fmt::Display;
use std::io::Write;
use std::process::ExitCode;
use std::io::Stdout;
use std::string::FromUtf8Error;
use std::sync::Arc;

use eyre::Error;

use color_eyre::Result;

use clap::Parser;

use num_traits::Num;

use agents_store::Store;
use agents_store::StoreExt;
use agents_store::tokio::LocalStore;

use quick_xml::Reader;
use quick_xml::Writer;
use quick_xml::events::BytesStart;
use quick_xml::events::Event as XmlEvent;

use atlas_tracing::TerminalTracingSubscriber;

use crossterm::cursor::DisableBlinking;
use crossterm::cursor::EnableBlinking;
use crossterm::event::DisableFocusChange;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableFocusChange;
use crossterm::event::EnableMouseCapture;
use crossterm::event::Event as CrosstermEvent;
use crossterm::event::KeyCode;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind as MouseAction;

use ratatui::TerminalOptions;
use ratatui::Viewport;
use ratatui::backend::CrosstermBackend;

use slate_surface::Surface;
use slate_scaffold::Scaffold;
use slate_style::Unit;
use slate_style::Size;
use slate_style::Flex;
use slate_style::FlexDirection;
use slate_style::BackgroundColor;
use slate_style::Border;

use slate_event::Click;
use slate_event::DoubleClick;

use slate_element::Text;
use slate_element::Element;
use slate_content::Content;

use slate_terminal_ratatui::RatatuiSurface;

//---
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// TODO
    #[arg(short, long, default_value = "trace")]
    log_level: String,
    
    /// TODO
    #[arg(long, default_value = "false")]
    console: bool,
    
    /// TODO
    #[arg(long, default_value = "true")]
    clear_before: bool,
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .with_thread_names(false)
        .with_line_number(false)
        .with_target(false)
        .with_file(false)
        .with_ansi(true)
        .without_time()
        .init();
    
    color_eyre::install()?;
    
    // The ultimate state here should be a managed Atlas State instance.
    // The Arc<RwLock<..>> stays here, and the String should be changed to
    // something like aninstance of  `atlas::State`.
    //  - In a real application, the inner state manages mutability internally.
    //  - Typically you'd only use the lock to replace the entire state instance.
    let terminal_store = LocalStore::<TerminalState, String>::new();
    
    let svg_content = std::fs::read_to_string("").unwrap_or_default();
    
    terminal_store.insert(&TerminalState::SvgContent, svg_content);
    
    let mut terminal_surface = RatatuiSurface::new(
        // Ratatui is doing some extra stuff for us here,
        // so if you're in an environment that may interfere
        // with crossterm's internal setup or management,
        // you'll likely want to init manually.
        ratatui::try_init_with_options({
            TerminalOptions {
                viewport: Viewport::Fullscreen,
                // viewport: Viewport::Inline(15),
                // viewport: Viewport::Fixed(Rect::new(0, 0, 100, 100)),
            }
        })?,
    );
    
    if !args.console && args.clear_before {
        terminal_surface.clear()?;
    }
    
    let stdout = terminal_surface.stdout();
    
    crossterm::execute!(&stdout, EnableFocusChange)?;
    crossterm::execute!(&stdout, EnableMouseCapture)?;
    crossterm::execute!(&stdout, EnableBlinking)?;
    
    let tracing_stream = terminal_store.get_or_init(&TerminalState::TracingStream);
    let tracing_subscriber = TerminalTracingSubscriber::new(tracing_stream);
    
    tracing::subscriber::with_default(tracing_subscriber, || -> Result<ExitCode> {
        loop {
            if terminal_surface.should_draw() {
                draw_terminal_surface(&terminal_store, &mut terminal_surface)?;
            }
            
            match dispatch_terminal_event(crossterm::event::read()?, &terminal_store) {
                TerminalAction::Exit(exit_code) => return Ok(exit_code),
                
                #[allow(unused)]
                unhandled_event => {
                    #[cfg(all(feature="dev", feature="verbose"))]
                    tracing::trace!("NoOp: {:?}", unhandled_event);
                }
            }
        }
    })?;
    
    terminal_surface.clear()?;
    
    crossterm::execute!(&stdout, DisableFocusChange)?;
    crossterm::execute!(&stdout, DisableMouseCapture)?;
    crossterm::execute!(&stdout, DisableBlinking)?;
    
    drop(terminal_surface);
    ratatui::restore();
    
    tracing::info!("Bye! <3");
    
    Ok(ExitCode::SUCCESS)
}

//---
pub struct ExampleApp {
    #[allow(unused)]
    store: LocalStore<TerminalState, String>,
}

impl ExampleApp {
    pub fn new() -> Self {
        ExampleApp {
            store: LocalStore::new(),
        }
    }
}

// TODO: #[derive(State)]
#[derive(Debug, Clone, Copy)]
#[derive(PartialEq, Eq, Hash)]
pub enum TerminalState {
    SvgContent,
    UserInput,
    TracingStream,
    UnhandledEvent,
    Error,
}

pub fn state_value_to_string(value: Arc<RwLock<String>>) -> String {
    match value.read().len() {
        0 => String::default(),
        len => value.read().to_owned(),
    }
}

#[derive(Default, Debug)]
pub enum TerminalAction {
    Exit(ExitCode),
    #[default]
    NoOp,
}

//---
#[derive(Default)]
pub struct Input<T>(pub T);

// TODO: Pls :(
// use bumpalo::collections::String;

impl<T: AsRef<str>> Element for Input<T> {
    fn draw(&self) -> impl FnOnce(Scaffold) -> Scaffold {
        let value = String::from(self.0.as_ref());
        
        move |input| {
            input
                .with_style(FlexDirection::Row)
                .with_slot::<InputIcon>(|icon, i| {
                    icon
                        .with_style(Size::width(2))
                        .with_content(Some("# "))
                })
                .with_slot::<InputValue<&str>>(|text, i| {
                    text
                        .with_style(FlexDirection::Row)
                        .with_style(Size::width(value.len()))
                        .with_content(Some(value))
                })
                .with_slot::<InputCursor>(|cursor, _| {
                    cursor
                        .with_style(BackgroundColor::from("#aaaaaaff"))
                        .with_style(Size::width(1))
                        .with_content(Some(" "))
                })
        }
    }
}

// TODO: Derive Icon for this type.
type InputIcon = &'static str;

#[derive(Default)]
struct InputValue<V> {
    value: V,
}

#[derive(Default)]
struct InputCursor;

// TODO: Move these to slate_scaffold and impl Slot for each.
struct Legend;

struct Header;

struct Body;

struct Footer;

fn draw_terminal_surface(
    store: &LocalStore<TerminalState, String>,
    surface: &mut RatatuiSurface<CrosstermBackend<Stdout>>,
) -> Result<()> {
    let work_dir = std::env::current_dir()
        .expect("Current Working Directory")
        .to_string_lossy()
        .into_owned();
    
    let some_event_state = store.get_or_init(&TerminalState::UserInput);
    
    let indexed_state = store.get_or_init(&TerminalState::UserInput);
    
    let send_user_message = {
        let user_input = store.get_or_init(&TerminalState::UserInput);
        
        move |_: &DoubleClick| {
            tracing::debug!("TODO: Send message; {:#?}", user_input);
        }
    };
    
    surface.draw(move |terminal| {
        terminal
            // Set events inline (for later handling by the runtime).
            .with_handler(send_user_message)
            .with_handler(|event: &Click| tracing::debug!("{:?} Event!", event))
            // Keys can be variant between state entries ..
            .with_state("some-key-value", some_event_state)
            .with_state(0, indexed_state)
            // Slot-based heirarchy for flexible tree-shaping.
            .with_slot::<Header>(|header, _| {
                header
                    .with_debug(false)
                    .with_style(FlexDirection::Row)
                    .with_style(Size::height(8))
                    .with_style(Border(Unit(1)))
                    .with_slot::<String>(|metadata, _| {
                        metadata
                            .with_element(Text("[Project Name]"))
                            .with_slot::<Legend>(|item, _| {
                                item
                                    .with_content(Some("Inner inner lol"))
                            })
                    })
                    .with_slot::<String>(|status, _| {
                        status
                            .with_style(Size::width(20))
                            .with_content(Some("Users:"))
                            .with_slot::<Legend>(|item, _| {
                                item
                                    // .with_style(Size::height(1))
                                    .with_content(Some("Inner inner lol"))
                            })
                            .with_slot::<&Legend>(|item, _| {
                                item
                                    .with_content(Some("Inner inner lol"))
                            })
                    })
                    .with_slot::<String>(|nav, _| {
                        nav
                            .with_style(FlexDirection::Column)
                            .with_style(Size::width(20))
                            .with_element(Text("Bots:"))
                            .with_slot::<Legend>(|item, _| {
                                item
                                    .with_content(Some("Inner inner lol"))
                            })
                    })
            })
            .with_slot::<Body>(|console, _| {
                let tracing_stream = store.get_or_init(&TerminalState::TracingStream);
                console
                    // TODO: Move to a dedicated tracing-stream
                    // crate with a "ui" feature for Slate types.
                    // .with_element(TracingStreamDisplay::from(&tracing_stream)).
                    .with_element(Text(tracing_stream.read().to_owned()))
            })
            .with_slot::<Footer>(|footer, i| {
                let source_content = store.get_or_init(&TerminalState::SvgContent);
                let parsed_content = render_svg(source_content.read().as_str())
                    .map_err(|error| tracing::error!("Failed to render SVG: {:}", error))
                    .unwrap_or_default();
                
                footer
                    .with_debug(true)
                    // .with_xray(true)
                    // .with_style(FlexDirection::Row)
                    // .with_style(Size::height(10))
                    .with_element(Text(parsed_content))
                    .with_slot::<Body>(repeat::<&str>(4, |row, i| {
                        row
                            .with_style(FlexDirection::Row)
                            .with_content(Some(format!("Row: {:}", i)))
                    }))
            })
            .with_slot::<Footer>(|terminal_io, _| {
                terminal_io
                    .with_style(FlexDirection::Column)
                    .with_style(Size::height(2))
                    .with_slot::<Text<&str>>(|cwd_display, _| {
                        cwd_display
                            .with_style(FlexDirection::Row)
                            .with_style(Size::height(1))
                            .with_element(Text(work_dir.to_string()))
                    })
                    .with_slot::<Input<&str>>(|input_field, _| {
                        let input_value = store.get(&TerminalState::UserInput)
                            .map(|value| value.read().to_owned())
                            .unwrap_or_default();
                        
                        input_field
                            .with_style(FlexDirection::Row)
                            .with_style(Size::height(input_value.lines().count()))
                            .with_element(Input(input_value))
                    })
            })
    })
}

pub fn repeat<S: 'static>(n: usize, draw_fn: impl Fn(Scaffold, usize) -> Scaffold) -> impl FnOnce(Scaffold, usize) -> Scaffold {
    move |mut parent_slot, _| {
        for i in 0..n {
            parent_slot = parent_slot.with_slot::<S>(|s, i| draw_fn(s, i));
        }
        
        parent_slot
    }
}

pub fn render_svg(content: &str) -> Result<String, Error> {
    let mut reader = Reader::from_str(&content);
    let mut writer = Writer::new(Vec::new()); // Write to a new Vec<u8>
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            XmlEvent::Start(tag) if tag.name().as_ref() == b"h1" => {
                let mut tag_output = BytesStart::new("h1");
                let attributes = tag.attributes().map(|attribute| {
                    match attribute {
                        Ok(attribute) => attribute,
                        Err(error) => {
                            let key = QName("$error".as_bytes());
                            let value = Cow::Owned(b"some_value".to_vec());
                            tracing::error!("Failed to read attribute: {:}", error);
                            Attribute { key, value }
                        }
                    }
                });
                
                tag_output.extend_attributes(attributes);
                
                tag_output.push_attribute(("class", "modified"));
                
                writer.write_event(XmlEvent::Start(tag_output))?;
            }
            XmlEvent::Eof => break,
            // For all other events, just write them out unmodified.
            event => {
                writer.write_event(event)?;
            }
        }
        
        buf.clear();
    }
    
    String::from_utf8(writer.into_inner())
        .map_err(Error::new)
}

fn dispatch_terminal_event(event: CrosstermEvent, store: &LocalStore::<TerminalState, String>) -> TerminalAction {
    use crossterm::event::Event as CrosstermEvent;
    
    let terminal_stream = store.get_or_init(&TerminalState::TracingStream);
    let history_len = { terminal_stream.read().lines().count() };
    let mut history_index = 0; // TODO: Get this from state (or else) ..
    
    let user_input = store.get_or_init(&TerminalState::UserInput);
    // let user_input_len = { terminal_stream.read().len() };
    
    match event {
        CrosstermEvent::Key(key) => match key.code {
            KeyCode::Esc => {
                return TerminalAction::Exit(ExitCode::SUCCESS);
            }
            KeyCode::Up => {
                if history_index < history_len {
                    history_index += 1;
                    
                    tracing::debug!("Getting history at index #{}", history_index);
                    
                    let terminal_stream = terminal_stream.read();
                    if let Some(history_item) = terminal_stream.lines().nth_back(history_index) {
                        let mut wlock = user_input.write();
                        *wlock = history_item.to_owned();
                    }
                }
            }
            KeyCode::Down => {
                if history_index == 0 {
                    user_input.write().clear();
                } else {
                    history_index -= 1;
                    
                    tracing::debug!("Getting history at index #{}", history_index);
                    
                    let terminal_stream = terminal_stream.read();
                    if let Some(line) = terminal_stream.lines().nth_back(history_index) {
                        let mut wlock = user_input.write();
                        *wlock = line.to_owned();
                    }
                }
            }
            KeyCode::Char(character) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    user_input.write().push(character);
                }
            }
            KeyCode::Backspace => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    user_input.write().pop();
                }
            }
            KeyCode::Enter => {
                let input_text = user_input.read();
                if input_text.len() > 0 {
                    match input_text.as_str() {
                        "clear" => {
                            terminal_stream.write().clear();
                        }
                        input_text => {
                            tracing::info!("{} {}", "@moodring.dev", input_text);
                        }
                    }
                }
                
                drop(input_text);
                user_input.write().clear();
            }
            unhandled_key => {
                #[cfg(all(feature="dev", feature="verbose"))]
                tracing::warn!("Unhandled key press: {:}", unhandled_key);
            }
        }
        CrosstermEvent::Mouse(event) => match event.kind {
            unhandled_event => {
                #[cfg(all(feature="dev", feature="verbose"))]
                tracing::warn!("Unhandled mouse event: {:}", unhandled_key);
            }
        }
        unhandled_event => {
            tracing::debug!("Unhandled event: {:?}", unhandled_event);
        }
    }
    
    return TerminalAction::NoOp;
}

//---
pub mod slate_draw {
    #![allow(unused)]
    
    use core::alloc::Allocator;

    //---
    #[derive(Debug, Clone, Copy)]
    pub struct DrawContext<A: Allocator> {
        pub(crate) arena: A,
    }

    impl<A: Allocator> DrawContext<A> {
        pub fn new_in(arena: A) -> Self {
            DrawContext {
                arena,
            }
        }
    }

    impl<A: Allocator + Copy> DrawContext<A> {
        pub fn arena(&self) -> A {
            self.arena
        }
    }
    
    pub trait DrawUpdate {
        //..
    }

    pub enum DrawReport<U: DrawUpdate> {
        Updates(Vec<U>),
        NoOp,
    }
}

//---
pub mod slate_surface {
    #![allow(unused)]
    
    use core::fmt::Debug;
    
    use color_eyre::Result;

    use crate::slate_draw::DrawUpdate;
    use crate::slate_scaffold::Scaffold;
    
    //---
    pub trait Surface<U> {
        fn draw<F>(&mut self, draw_fn: F) -> Result<()>
        where
            F: for<'a> FnOnce(Scaffold<'a>) -> Scaffold<'a>;
    }

    pub struct SurfaceUpdate {
        //..
    }
    
    impl DrawUpdate for SurfaceUpdate {
        //..
    }
}

//---
pub mod slate_scaffold {
    #![allow(unused)]
    
    use core::any::Any;
    use core::hash::Hash;
    use core::fmt::Debug;
    use core::any::TypeId;
    use std::alloc::Allocator;
    use std::hash::DefaultHasher;
    use std::hash::Hasher;
    
    use hashbrown::DefaultHashBuilder;
    use hashbrown::HashMap;
    
    use bumpalo::Bump;
    use bumpalo::boxed::Box as BBox;
    use bumpalo::collections::Vec as BVec;
    use bumpalo::collections::String as BString;
    
    use crate::slate_collections::OrderedMap;
    use crate::slate_draw::DrawContext;
    use crate::slate_content::Content;
    use crate::slate_element::Element;
    use crate::slate_style::Style;
    use crate::slate_style::StyleSheet;
    use crate::slate_style::StyleValue;
    
    #[derive(Debug)]
    pub struct EventHandler<A: Allocator>(Box<dyn Any, A>);
    
    #[derive(Debug)]
    pub struct EventStack<A: Allocator> {
        handlers: HashMap<TypeId, Vec<EventHandler<A>, A>, DefaultHashBuilder, A>,
    }
    
    impl<A: Allocator> EventStack<A> {
        pub fn new_in(arena: A) -> Self {
            EventStack {
                handlers: HashMap::new_in(arena),
            }
        }
    }

    #[derive(Debug)]
    pub struct Scaffold<'ctx> {
        pub debug: bool,
        context: DrawContext<&'ctx Bump>,
        element: Option<&'ctx dyn Any>,
        content: Option<BString<'ctx>>,
        styles: StyleSheet<&'ctx Bump>,
        events: EventStack<&'ctx Bump>,
        state: OrderedMap<(TypeId, usize), &'ctx dyn Any, DefaultHashBuilder, &'ctx Bump>,
        slots: OrderedMap<(TypeId, usize), Scaffold<'ctx>, DefaultHashBuilder, &'ctx Bump>,
    }
    
    impl<'ctx> Scaffold<'ctx> {
        pub fn new_in(arena: &'ctx Bump) -> Self {
            Scaffold {
                debug: false,
                context: DrawContext::new_in(arena),
                element: None,
                content: None,
                styles: StyleSheet::new_in(arena),
                events: EventStack::new_in(arena),
                state: OrderedMap::new_in(arena),
                slots: OrderedMap::new_in(arena),
            }
        }
    }
    
    impl<'ctx> Scaffold<'ctx> {
        pub fn alloc<V>(&self, value: V) -> &'ctx V {
            self.context.arena().alloc(value)
        }
        
        pub fn with_element<E: Element + Any + 'ctx>(mut self, element: E) -> Self {
            self = self.with_slot::<DefaultSlot>(|slot, _| element.draw()(slot));
            
            self.content = element.content().map(|c| BString::from_str_in(c, self.context.arena()));
            self.element = Some(self.alloc(element));
            
            self // etc..
        }
        
        pub fn with_content<C: AsRef<str> + 'ctx>(mut self, content: Option<C>) -> Self {
            if let Some(content) = content {
                self.content = Some(BString::from_str_in(content.as_ref(), self.context.arena()));
            }
            self // etc..
        }
        
        // TODO: Move the "Any" requirement to a method on a "State" type.
        pub fn with_state<K: Hash + Any, V: Any>(mut self, key: K, value: V) -> Self {
            let mut hasher = DefaultHasher::default();
            let hash = key.hash(&mut hasher);
            self.state.insert((TypeId::of::<K>(), hasher.finish() as usize), self.alloc(value));
            self // etc..
        }
        
        pub fn with_style<S: Into<StyleValue> + Any>(mut self, style: S) -> Self {
            self.styles.insert(style);
            self // etc..
        }
        
        pub fn with_handler<E: Any>(mut self, handler: impl Fn(&E) + 'static) -> Self {
            self.events
                .handlers
                .entry(TypeId::of::<E>())
                .or_insert_with(|| Vec::new_in(self.context.arena()))
                .push(EventHandler(Box::new_in(handler, self.context.arena())));
            self // etc..
        }
        
        pub fn with_slot<S: Any + 'ctx>(mut self, draw_fn: impl FnOnce(Scaffold<'ctx>, usize) -> Scaffold<'ctx>) -> Self {
            let next_idx = self.slots.len();
            let slot = Scaffold::new_in(self.context.arena());
            self.slots.insert((TypeId::of::<S>(), next_idx), draw_fn(slot, next_idx));
            self // etc..
        }
        
        pub fn with_slot_at<S: Any + 'ctx>(mut self, i: usize, draw_fn: impl FnOnce(Scaffold<'ctx>, usize) -> Scaffold<'ctx>) -> Self {
            let slot = Scaffold::new_in(self.context.arena());
            self.slots.insert((TypeId::of::<S>(), i), draw_fn(slot, i));
            self // etc..
        }
        
        pub fn with_debug(mut self, xray: bool) -> Self {
            self.debug = xray;
            self // etc..
        }
    }

    impl<'ctx> Scaffold<'ctx> {
        pub fn set_state(&mut self, debug: bool) -> &Self {
            self.debug = debug;
            self // etc..
        }
        
        pub fn set_debug(&mut self, debug: bool) -> &Self {
            self.debug = debug;
            self // etc..
        }
    }

    impl<'ctx> Scaffold<'ctx> {
        pub fn context(&self) -> DrawContext<&Bump> {
            self.context
        }
        
        pub fn arena(&self) -> &Bump {
            self.context.arena()
        }
        
        pub fn element<E: Element + Any>(&self) -> Option<&E> {
            self.element.and_then(|element| element.downcast_ref::<E>())
        }
        
        pub fn content(&self) -> Option<&BString<'ctx>> {
            self.content.as_ref()
        }
        
        pub fn styles(&self) -> &StyleSheet<&Bump> {
            &self.styles
        }
        
        pub fn handlers(&self) -> &EventStack<&Bump> {
            &self.events
        }
        
        pub fn state(&self) -> &OrderedMap<(TypeId, usize), &dyn Any, DefaultHashBuilder, &Bump> {
            &self.state
        }
        
        pub fn slots(&self) -> &OrderedMap<(TypeId, usize), Scaffold<'_>, DefaultHashBuilder, &Bump> {
            &self.slots
        }
        
        pub fn slot<S: Any>(&self, i: usize) -> Option<&Scaffold<'_>> {
            self.slots.get(&(TypeId::of::<S>(), i))
        }
        
        pub fn debug(&self) -> bool {
            self.debug
        }
    }
    
    //---
    pub struct DefaultSlot;
}

//---
pub mod slate_element {
    #![allow(unused)]
    
    use alloc::sync::Arc;

    use core::any::Any;
    use core::fmt::Display;
    use core::fmt::Debug;
    use core::fmt::Formatter;
    use core::fmt::Error as FmtError;
    
    use derive_more::Display;
    
    use parking_lot::RwLock;

    use crate::slate_scaffold::Scaffold;
    use crate::slate_content::Content;
    
    //--
    pub trait Element {
        fn init(&self) -> Meta {
            Meta::default()
        }
        
        fn content(&self) -> Option<&str> {
            None
        }
        
        fn draw(&self) -> impl FnOnce(Scaffold) -> Scaffold {
            |s| s
        }
    }
    
    #[derive(Default, Debug)]
    pub struct Meta {
        //
    }
    
    #[derive(Default, Display, Debug)]
    pub struct Text<C>(pub C);
    
    impl<C: AsRef<str>> Element for Text<C> {
        fn content(&self) -> Option<&str> {
            Some(self.0.as_ref())
        }
    }
}

//---
pub mod slate_content {
    use num_traits::FromPrimitive;
    use num_traits::Num;
    use parking_lot::RwLock;

    use alloc::sync::Arc;

    use core::fmt::Display;
    use core::fmt::Debug;
    use core::fmt::Formatter;
    use core::fmt::Error as FmtError;

    pub trait Content: Debug {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError>;
    }
    
    impl<C: Content> Content for &C {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Content::fmt(*self, f)
        }
    }
    
    impl Display for &dyn Content {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Content::fmt(*self, f)
        }
    }

    impl Content for &str {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
            f.write_str(self)
        }
    }

    impl Content for String {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
            f.write_str(self.as_str())
        }
    }
    
    impl Content for Arc<String> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
            f.write_str(self.as_str())
        }
    }
    
    impl Content for Arc<RwLock<String>> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
            f.write_str(self.read().as_str())
        }
    }
    
    impl Content for Option<Arc<RwLock<String>>> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
            match self {
                Some(content) => f.write_str(content.read().as_str()),
                None => f.write_str("<empty>"),
            }
        }
    }

    //--
    /// A custom writer that counts lines without allocating memory.
    /// Disclaimer: AI Slop; Do not use this implementation.
    pub struct LineCounter<T> {
        /// The number of newline characters (`\n`) encountered.
        newlines: T,
        /// Tracks if any characters have been written at all.
        is_empty: bool,
    }

    impl<T: Default> LineCounter<T> {
        /// Creates a new LineCounter.
        pub fn new() -> Self {
            LineCounter {
                newlines: T::default(),
                is_empty: true,
            }
        }
    }

    impl<T: Num + Copy> LineCounter<T> {
        /// Calculates the final line count based on the number of newlines.
        /// An empty string has 0 lines.
        /// A non-empty string has `newlines + 1` lines.
        pub fn count(&self) -> T {
            if self.is_empty {
                T::zero()
            } else {
                self.newlines + T::one()
            }
        }
    }

    // The core logic is here!
    impl<T: Num + Copy + FromPrimitive> core::fmt::Write for LineCounter<T> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            if !s.is_empty() {
                self.is_empty = false;
            }
            
            if let Some(num) = T::from_usize(s.matches('\n').count()) {
                self.newlines = self.newlines + num;
            }
            
            Ok(())
        }
    }
}

//---
pub mod slate_style {
    #![allow(unused)]
    
    use core::alloc::Allocator;
    use core::any::TypeId;
    use core::fmt::Debug;
    use std::any::Any;
    
    use derive_more::*;
    
    use enum_dispatch::*;
    
    use hashbrown::HashMap;
    use hashbrown::DefaultHashBuilder;
    use num_traits::AsPrimitive;
    use num_traits::Num;
    
    //---
    #[enum_dispatch]
    pub trait Style: TryInto<StyleValue> {
        //..
    }
    
    #[derive(Debug, Index, IndexMut, Deref, DerefMut)]
    pub struct StyleSheet<A: Allocator> {
        #[index] #[index_mut] #[deref] #[deref_mut]
        styles: HashMap<TypeId, Vec<StyleValue, A>, DefaultHashBuilder, A>,
        arena: A,
    }
    
    impl<A: Allocator + Copy> StyleSheet<A> {
        pub fn new_in(arena: A) -> Self {
            StyleSheet {
                styles: HashMap::new_in(arena),
                arena,
            }
        }
    }
    
    impl<A: Allocator + Copy> StyleSheet<A> {
        pub fn insert<S: Into<StyleValue> + Any>(&mut self, style: S) -> &Self {
            self.styles
                .entry(style.type_id())
                .or_insert_with(|| Vec::new_in(self.arena))
                .push(style.into());
            self
        }
    }

    #[derive(Debug)]
    #[enum_dispatch(Style)]
    pub enum StyleValue {
        BackgroundColor(BackgroundColor),
        Flex(Flex),
        FlexDirection(FlexDirection),
        Size(Size),
        Heading(Heading),
        Border(Border),
    }
    
    //---
    #[derive(Clone, Copy, Default, Debug)]
    pub struct BackgroundColor {
        //..
    }
    
    impl BackgroundColor {
        pub fn new() -> Self {
            BackgroundColor {
                //..
            }
        }
    }
    
    impl<C: Into<String>> From<C> for BackgroundColor {
        fn from(color: C) -> Self {
            BackgroundColor {
                //..
            }
        }
    }
    
    impl Style for BackgroundColor {
        //..
    }
    
    //---
    #[derive(Clone, Copy, Default, Debug)]
    pub struct Flex(pub Unit);
    
    impl Style for Flex {
        //..
    }
    
    #[derive(Clone, Copy, Default, Debug, IsVariant)]
    pub enum FlexDirection {
        #[default]
        Column,
        Row,
    }
    
    impl Style for FlexDirection {
        //..
    }
    
    #[derive(Display, Debug, Clone, Copy, Default, From)]
    pub struct Unit(pub usize);
    
    impl AsPrimitive<u8> for Unit {
        fn as_(self) -> u8 {
            self.0.as_()
        }
    }
    
    impl AsPrimitive<i8> for Unit {
        fn as_(self) -> i8 {
            self.0.as_()
        }
    }
    
    impl AsPrimitive<u16> for Unit {
        fn as_(self) -> u16 {
            self.0.as_()
        }
    }
    
    impl AsPrimitive<i16> for Unit {
        fn as_(self) -> i16 {
            self.0.as_()
        }
    }
    
    impl AsPrimitive<u32> for Unit {
        fn as_(self) -> u32 {
            self.0.as_()
        }
    }
    
    impl AsPrimitive<i32> for Unit {
        fn as_(self) -> i32 {
            self.0.as_()
        }
    }
    
    impl AsPrimitive<f32> for Unit {
        fn as_(self) -> f32 {
            self.0.as_()
        }
    }
    
    impl AsPrimitive<u64> for Unit {
        fn as_(self) -> u64 {
            self.0.as_()
        }
    }
    
    impl AsPrimitive<i64> for Unit {
        fn as_(self) -> i64 {
            self.0.as_()
        }
    }
    
    impl AsPrimitive<f64> for Unit {
        fn as_(self) -> f64 {
            self.0.as_()
        }
    }
    
    impl AsPrimitive<usize> for Unit {
        fn as_(self) -> usize {
            self.0.as_()
        }
    }
    
    #[derive(Default, Display, Debug, Clone, Copy)]
    pub enum Value {
        #[default]
        Auto,
        Px(Unit),
        Fill(Unit),
        Percent(Unit),
    }
    
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Size(pub Value, pub Value, pub Value);
    
    impl Size {
        pub fn width<V: Into<Unit>>(value: V) -> Self {
            Size(Value::Px(value.into()), Value::Auto, Value::Auto)
        }
        
        pub fn x<V: Into<Unit>>(value: V) -> Self {
            Size::width(value)
        }
        
        pub fn height<V: Into<Unit>>(value: V) -> Self {
            Size(Value::Auto, Value::Px(value.into()), Value::Auto)
        }
        
        pub fn y<V: Into<Unit>>(value: V) -> Self {
            Size::height(value)
        }
        
        pub fn depth<V: Into<Unit>>(value: V) -> Self {
            Size(Value::Auto, Value::Auto, Value::Px(value.into()))
        }
        
        pub fn z<V: Into<Unit>>(value: V) -> Self {
            Size::depth(value)
        }
        
        pub fn rect<V: Into<Unit>>(x: V, y: V) -> Self {
            Size(Value::Px(x.into()), Value::Px(y.into()), Value::Auto)
        }
        
        pub fn xy<V: Into<Unit>>(x: V, y: V) -> Self {
            Size::rect(x, y)
        }
        
        pub fn cube<V: Into<Unit>>(x: V, y: V, z: V) -> Self {
            Size(Value::Px(x.into()), Value::Px(y.into()), Value::Px(z.into()))
        }
        
        pub fn xyz<V: Into<Unit>>(x: V, y: V, z: V) -> Self {
            Size::cube(x, y, z)
        }
    }
    
    impl Style for Size {
        //..
    }
    
    #[derive(Clone, Copy, Debug)]
    pub enum Heading {
        // Level(usize),
        Primary,
    }
    
    impl Style for Heading {
        //..
    }
    
    #[derive(Clone, Copy, Default, Debug)]
    pub struct Border(pub Unit);
    
    impl Style for Border {
        //..
    }
}

//---
pub mod slate_event {
    use std::fmt::Debug;
    
    pub trait Event: Debug {
        //..
    }
    
    #[derive(Debug)]
    pub struct Click;
    impl Event for Click {
        //..
    }

    #[derive(Debug)]
    pub struct DoubleClick;
    impl Event for DoubleClick {
        //..
    }

    #[derive(Debug)]
    pub struct Scroll;
    impl Event for Scroll {
        //..
    }
}

pub mod slate_collections {
    //! Disclaimer: This is AI slop. I didn't want to write the ordered map by
    //! hand, but this should be replaced with a human-written impl.
    //! 
    //! Note: if you see this in the public repo, pls open a PR and berate
    //! the repo maintainer (or whoever).
    
    use alloc::alloc::Allocator;
    use alloc::alloc::Global;
    
    use core::borrow::Borrow;
    use core::hash::Hash;
    use core::hash::BuildHasher;
    
    use hashbrown::HashMap;
    use hashbrown::DefaultHashBuilder;
    
    /// A map-like data structure that preserves the original insertion order of its keys.
    ///
    /// It is implemented using a `Vec` to store the ordered keys and a `HashMap`
    /// for fast O(1) lookups.
    #[derive(Debug)]
    pub struct OrderedMap<K, V, S = DefaultHashBuilder, A: Allocator = Global> {
        keys: Vec<K, A>,
        map: HashMap<K, V, S, A>,
    }

    impl<K, V> OrderedMap<K, V>
    where
        K: Eq + Hash + Clone,
    {
        /// Creates a new, empty `OrderedMap` with the global allocator.
        pub fn new() -> Self {
            Self {
                keys: Vec::new(),
                map: HashMap::new(),
            }
        }
    }

    impl<K, V, A> OrderedMap<K, V, DefaultHashBuilder, A>
    where
        K: Eq + Hash,
        A: Allocator + Copy,
    {
        /// Creates a new, empty `OrderedMap` with the given allocator.
        pub fn new_in(allocator: A) -> Self {
            Self {
                keys: Vec::new_in(allocator),
                map: HashMap::<K, V, DefaultHashBuilder, A>::new_in(allocator),
            }
        }
    }
    
    // Default implementation
    impl<K, V> Default for OrderedMap<K, V>
    where
        K: Eq + Hash + Clone,
    {
        fn default() -> Self {
            Self::new()
        }
    }
    
    impl<K, V, S, A> OrderedMap<K, V, S, A>
    where
        K: Eq + Hash + Clone,
        A: Allocator,
        S: BuildHasher,
    {
        /// Inserts a key-value pair into the map.
        ///
        /// If the map did not have this key present, `None` is returned. If the map
        /// did have this key present, the value is updated, and the old value is
        /// returned. The key is not updated, and insertion order is not changed.
        pub fn insert(&mut self, key: K, value: V) -> Option<V> {
            if !self.map.contains_key(&key) {
                self.keys.push(key.clone());
            }
            self.map.insert(key, value)
        }

        /// Returns a reference to the value corresponding to the key.
        pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
        where
            K: Borrow<Q>,
            Q: Hash + Eq,
        {
            self.map.get(key)
        }

        /// Removes a key from the map, returning the value at the key if the key
        /// was previously in the map.
        ///
        /// This is an O(n) operation because it requires finding and removing the
        /// key from the internal `Vec`.
        pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
        where
            K: Borrow<Q>,
            Q: Hash + Eq,
        {
            let value = self.map.remove(key);
            if value.is_some() {
                // Find the key in the `keys` Vec and remove it.
                if let Some(index) = self.keys.iter().position(|k| k.borrow() == key) {
                    self.keys.remove(index);
                }
            }
            value
        }

        /// Returns the number of elements in the map.
        pub fn len(&self) -> usize {
            self.keys.len()
        }

        /// Returns `true` if the map contains no elements.
        pub fn is_empty(&self) -> bool {
            self.keys.is_empty()
        }

        /// Returns an iterator visiting all key-value pairs in insertion order.
        pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
            self.keys.iter().map(move |key| (key, &self.map[key]))
        }

        /// Returns an iterator visiting all keys in insertion order.
        pub fn keys(&self) -> impl Iterator<Item = &K> {
            self.keys.iter()
        }

        /// Returns an iterator visiting all values in insertion order.
        pub fn values(&self) -> impl Iterator<Item = &V> {
            self.keys.iter().map(move |key| &self.map[key])
        }
    }
}

//---
pub mod slate_terminal {
    #![allow(unused)]
    
    use crate::slate_surface::Surface;
    use crate::slate_scaffold::Scaffold;
    
    pub struct TerminalApp<S> {
        surface: S,
    }
    
    pub struct TerminalError {
        //..
    }
}

//---
// Note: for BevySurface:
//  - Build a Scaffold.
//  - Compute changes since last update.
//  - Iterate changes:
//     - Find the Current element from the reactive surface.

//---
pub mod slate_terminal_ratatui {
    #![allow(unused)]
    
    use std::any::Any;
    use std::any::TypeId;
    use std::fmt::Display;
    use std::fmt::Write;
    use std::io::Stderr;
    use std::io::Stdin;
    use std::io::Stdout;
    use std::io::Error as IoError;
    
    use bumpalo::Bump;
    use bumpalo_herd::Herd;
    use bumpalo_herd::Member;
    
    use color_eyre::owo_colors::OwoColorize;
    use color_eyre::Result;
    
    use derive_more::Deref;
    use derive_more::DerefMut;
    use num_traits::AsPrimitive;
    
    use ansi_to_tui::IntoText;
    
    use ratatui::layout::Flex;
    use ratatui::layout::Position;
    use ratatui::layout::Rect;
    use ratatui::symbols::border;
    use ratatui::text::ToText;
    use ratatui::widgets::block::Title;
    use ratatui::CompletedFrame;
    use ratatui::Terminal;
    use ratatui::Frame;
    use ratatui::text::Line;
    use ratatui::text::ToLine;
    use ratatui::text::Text;
    use ratatui::widgets::List;
    use ratatui::widgets::ListDirection;
    use ratatui::widgets::BorderType;
    use ratatui::widgets::Borders;
    use ratatui::widgets::Block;
    use ratatui::widgets::Paragraph;
    use ratatui::widgets::Clear;
    use ratatui::layout::Layout;
    use ratatui::layout::Constraint;
    use ratatui::layout::Direction;
    use ratatui::layout::Alignment;
    use ratatui::style::Style;
    use ratatui::style::Styled;
    use ratatui::style::Stylize;
    use ratatui::style::Modifier;
    use ratatui::style::Color;
    use ratatui::backend::Backend;
    use ratatui::backend::CrosstermBackend;

    use crate::slate_content::LineCounter;
    use crate::slate_style::Border;
    use crate::slate_style::FlexDirection;
    use crate::slate_style::Heading;
    use crate::slate_style::Size;
    use crate::slate_style::StyleValue;
    use crate::slate_style::Value;
    use crate::slate_surface::Surface;
    use crate::slate_scaffold::Scaffold;

    //---
    #[derive(Deref, DerefMut)]
    pub struct RatatuiSurface<B: Backend> {
        #[deref]
        #[deref_mut]
        terminal: Terminal<B>,
        allocator: Herd,
    }
    
    impl<B: Backend + core::fmt::Debug> core::fmt::Debug for RatatuiSurface<B> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("RatatuiSurface")
                .field("terminal", &self.terminal)
                .finish()
        }
    }

    impl<B: Backend> RatatuiSurface<B> {
        pub fn new(terminal: Terminal<B>) -> Self {
            RatatuiSurface {
                terminal,
                allocator: Herd::new(),
            }
        }
    }

    impl<B: Backend> RatatuiSurface<B> {
        pub fn should_draw(&self) -> bool {
            true
        }
    }

    impl<B: Backend> RatatuiSurface<B> {
        pub fn clear(&mut self) -> Result<(), IoError> {
            self.terminal.clear()
        }
    }

    impl RatatuiSurface<CrosstermBackend<Stdout>> {
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

    impl Surface<u16> for RatatuiSurface<CrosstermBackend<Stdout>> {
        fn draw<F>(&mut self, draw_fn: F) -> Result<()>
        where
            F: for<'a> FnOnce(Scaffold<'a>) -> Scaffold<'a>,
        {
            let member = self.allocator.get();
            let arena = member.as_bump();
            
            let scaffold = draw_fn(Scaffold::new_in(arena));
            
            // Walk the scaffold and apply optimizations:
            //  - Unpack event handlers (where possible).
            //  - Build/validate/normalize styles.
            //  - Save a snapshot of each item.
            //.
            // Optionally apply retained-mode rules:
            //  - Find and apply Node ids (lookup based on index + hash).
            //  - Nodes with changes should be marked.
            
            let completed_frame = self.terminal.draw(|frame| {
                Self::render(&scaffold, frame, &frame.area());
            })?;
            
            Ok(())
        }
    }
    
    impl RatatuiSurface<CrosstermBackend<Stdout>> {
        fn render(scaffold: &Scaffold, frame: &mut Frame, frame_bounds: &Rect) {
            let mut content_bounds = *frame_bounds;
            let mut border_size = 0;
            
            if scaffold.debug() {
                // TODO: Draw debug information on this!
                let debug_block = Block::default()
                    .title_top(Line::from(format!("Pos: {}x{}", content_bounds.top(), content_bounds.left())).alignment(Alignment::Left))
                    .title_bottom(Line::from(format!("Size: {}x{}", content_bounds.width, content_bounds.height)).alignment(Alignment::Right))
                    .border_style(Style::new().dim())
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL);
                
                let debug_inner_area = debug_block.inner(content_bounds);
                
                frame.render_widget(debug_block, *frame_bounds);
                
                content_bounds = debug_inner_area;
                border_size += 1;
            }
            
            if let Some(borders) = scaffold.styles().get(&TypeId::of::<Border>()) {
                for border in borders.iter() {
                    if let StyleValue::Border(Border(border_width)) = border {
                        let border_block = Block::default()
                            // .title_top(Line::from("TODO: Border labels ..").alignment(Alignment::Left))
                            .border_style(Style::new())
                            .border_type(BorderType::Rounded)
                            .borders(Borders::ALL);
                        
                        let border_inner_area = border_block.inner(content_bounds);
                        
                        frame.render_widget(border_block, content_bounds);
                        
                        content_bounds = border_inner_area;
                        border_size += 1;
                    }
                }
            }
            
            //--
            let flex_direction = scaffold.styles().iter()
                .find(|(type_id, values)| **type_id == TypeId::of::<FlexDirection>())
                .map(|(_, values)| match values.first() {
                    Some(StyleValue::FlexDirection(direction)) => *direction,
                    _ => FlexDirection::Column,
                })
                .unwrap_or_default();
            
            let mut layout_constraints = Vec::with_capacity(scaffold.slots().len() + 1);
            
            if let Some(content) = scaffold.content() {
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
            for (_, slot) in scaffold.slots().iter() {
                layout_constraints.push({
                    slot.styles()
                        .get(&TypeId::of::<Size>())
                        .and_then(|values| {
                            let is_column = flex_direction.is_column();
                            let is_row = flex_direction.is_row();
                            values.iter()
                                .find(|value| matches!(value, StyleValue::Size(..)))
                                .and_then(|value| match value {
                                    StyleValue::Size(Size(Value::Px(width), ..)) if is_row => {
                                        Some(Constraint::Length(width.as_()))
                                    }
                                    StyleValue::Size(Size(_, Value::Px(height), ..)) if is_column => {
                                        Some(Constraint::Length(height.as_()))
                                    }
                                    _ => None
                                })
                        })
                        .unwrap_or(Constraint::Min(0))
                });
            }
            
            let layout = Layout::new(Direction::from(flex_direction), layout_constraints);
            
            //--
            let layout_areas = layout.split(content_bounds);
            let mut slot_areas = layout_areas.into_iter();
            let content_area = scaffold.content()
                .and_then(|_| slot_areas.next());
            
            if let Some(content_area) = content_area && let Some(content) = scaffold.content() {
                if *content_area != *frame_bounds {
                    // tracing::debug!("Outer: {}; Inner: {}", frame_area, inner_area);
                }
                
                // TODO: Use the content area calculated above.
                let mut text = Text::default();
                let mut style = Style::default();
                
                // let pos = content_area.as_position();
                // let size = content_area.as_size();
                let widget = match content.to_string().into_text() {
                    Ok(content_text) => Paragraph::new(content_text).style(style),
                    Err(error) => {
                        tracing::error!("Failed to get rich text from content: {}", error);
                        Paragraph::new("ERROR".red())
                    }
                };
                
                frame.render_widget(widget, *content_area);
            }
            
            for (i, area) in slot_areas.enumerate() {
                match scaffold.slots().values().nth(i) {
                    Some(slot) => Self::render(slot, frame, area),
                    None => tracing::warn!("Failed to get slot {}!", i),
                }
            }
        }
    }
    
    impl From<FlexDirection> for Direction {
        fn from(flex_direction: FlexDirection) -> Self {
            match flex_direction {
                FlexDirection::Column => Direction::Vertical,
                FlexDirection::Row => Direction::Horizontal,
            }
        }
    }
    
    fn size_to_constraint(size: &Size, direction: FlexDirection, area: &Rect) -> Constraint {
        match size {
            Size(Value::Px(width), _, _) => Constraint::Length(width.as_()),
            Size(_, Value::Px(height), _) => Constraint::Length(height.as_()),
            Size(_, _, _) => match direction {
                FlexDirection::Row => Constraint::Length(area.width),
                FlexDirection::Column => Constraint::Length(area.height),
            }
        }
    }
}

//---
pub mod atlas_tracing {
    //! Disclaimer: This is (lightly modified) AI slop. I didn't want to write
    //! the subscriber by hand while solving Slate's core problems, so I took
    //! a shortcut. This is not intended to make it to the public repo.
    //! 
    //! Note: if you see this in the public repo, pls open a PR and berate
    //! the repo maintainer (or whoever).
    // #![allow(unused)]
    
    /// Danger: Not to be trusted.
    #[cfg(not(debug_assertions))]
    compile_error!("DO NOT USE THIS; AI-GENERATED SLOP");

    use std::collections::VecDeque;
    use std::fmt::Write;
    use std::sync::Arc;

    use parking_lot::Mutex;
    use parking_lot::RwLock;
    
    use color_eyre::owo_colors::OwoColorize;
    
    use crossterm::style::Stylize;
    
    use tracing::Event;
    use tracing::Level;
    use tracing::Metadata;
    use tracing::field::Field;
    use tracing::field::FieldSet;
    use tracing::field::Visit;
    use tracing::span::Attributes;
    use tracing::span::Id;
    
    use tracing_core::subscriber::Subscriber
        as TracingSubscriber;
    use tracing_core::callsite::Identifier;
    use tracing_core::span::Current;
    use tracing_core::span::Record;
    use tracing_core::Kind;

    //---
    pub struct TerminalTracingSubscriber {
        state: Arc<RwLock<String>>,
    }
    
    impl TerminalTracingSubscriber {
        pub fn new(state: Arc<RwLock<String>>) -> Self {
            TerminalTracingSubscriber {
                state,
            }
        }
    }

    impl TracingSubscriber for TerminalTracingSubscriber {
        fn enabled(&self, metadata: &Metadata<'_>) -> bool {
            metadata.level() <= &Level::TRACE
        }

        fn new_span(&self, attrs: &Attributes<'_>) -> Id {
            let id = Id::from_u64(0);
            let metadata = attrs.metadata();
            let parent = if let Some(parent) = attrs.parent() {
                format!(" with parent span {}", parent.into_u64())
            } else if attrs.is_root() {
                " as root span".to_string()
            } else {
                "".to_string()
            };
            
            println!("{} ({}) {}", metadata.name(), metadata.level(), parent);
            
            id
        }

        fn record(&self, span: &Id, values: &Record<'_>) {
            let mut visitor = TerminalTracingSpanVisitor;
            println!("Span {} recorded new values:", span.into_u64());
            values.record(&mut visitor);
        }

        fn record_follows_from(&self, span: &Id, follows: &Id) {
            //..
        }

        fn event(&self, event: &Event<'_>) {
            if let Some(mut state) = self.state.try_write() {
                let level = match *event.metadata().level() {
                    Level::ERROR => "EROR".red(),
                    Level::WARN => "WARN".yellow(),
                    Level::INFO => "INFO".blue(),
                    Level::DEBUG => "DEBG".magenta(),
                    Level::TRACE => "TRCE".cyan(),
                };
                
                let target = event.metadata().target();
                let mut msg = String::new();

                let mut visitor = TerminalTracingStringVisitor(&mut msg);
                event.record(&mut visitor);
                state.push_str(format!("{} {} {}\n", level, target.dim(), msg).as_str());
            }
        }

        fn enter(&self, id: &Id) {
            println!("Entering span {}", id.into_u64());
        }
        
        fn exit(&self, id: &Id) {
            println!("Exiting span {}", id.into_u64());
        }

        fn current_span(&self) -> Current {
            let span = todo!("self.current_span"); // Option<Span>
            let callsite = Identifier(todo!("callsite??"));
            
            // TODO: let id = Id::from_u64(todo!("Get the id from span."));
            Current::new(todo!("span.id"), {
                &Metadata::new(
                    "name",
                    "target",
                    Level::DEBUG,
                    Some("filename.blah"),
                    Some(0),
                    Some("module::path"),
                    FieldSet::new(&[], callsite),
                    Kind::SPAN,
                )
            })
        }

        fn clone_span(&self, id: &Id) -> Id {
            id.clone()
        }
    }

    pub struct TerminalTracingSpanVisitor;

    impl Visit for TerminalTracingSpanVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            println!("{}: {:?}", field.name(), value);
        }
    }

    pub struct TerminalTracingStringVisitor<'a>(&'a mut String);

    impl<'a> Visit for TerminalTracingStringVisitor<'a> {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                write!(self.0, "{:?}", value).unwrap();
            } else {
                write!(self.0, " {}: {:?}", field.name(), value).unwrap();
            }
        }
    }
}
