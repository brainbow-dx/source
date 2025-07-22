#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2024::*;
#[macro_use]
extern crate std;
extern crate alloc;
use std::process::ExitCode;
struct Scaffold;
#[automatically_derived]
impl ::core::fmt::Debug for Scaffold {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "Scaffold")
    }
}
struct TerminalSurface;
#[automatically_derived]
impl ::core::default::Default for TerminalSurface {
    #[inline]
    fn default() -> TerminalSurface {
        TerminalSurface {}
    }
}
impl TerminalSurface {
    fn draw(
        &mut self,
        _draw_fn: impl Fn(&mut Scaffold) -> Result<(), ()>,
    ) -> Result<(), EtchError> {
        Ok(())
    }
}
fn main() -> anyhow::Result<ExitCode> {
    tracing_subscriber::fmt::init();
    {
        use ::tracing::__macro_support::Callsite as _;
        static __CALLSITE: ::tracing::callsite::DefaultCallsite = {
            static META: ::tracing::Metadata<'static> = {
                ::tracing_core::metadata::Metadata::new(
                    "event examples\\etch.rs:26",
                    "etch",
                    ::tracing::Level::INFO,
                    ::tracing_core::__macro_support::Option::Some("examples\\etch.rs"),
                    ::tracing_core::__macro_support::Option::Some(26u32),
                    ::tracing_core::__macro_support::Option::Some("etch"),
                    ::tracing_core::field::FieldSet::new(
                        &["message"],
                        ::tracing_core::callsite::Identifier(&__CALLSITE),
                    ),
                    ::tracing::metadata::Kind::EVENT,
                )
            };
            ::tracing::callsite::DefaultCallsite::new(&META)
        };
        let enabled = ::tracing::Level::INFO
            <= ::tracing::level_filters::STATIC_MAX_LEVEL
            && ::tracing::Level::INFO <= ::tracing::level_filters::LevelFilter::current()
            && {
                let interest = __CALLSITE.interest();
                !interest.is_never()
                    && ::tracing::__macro_support::__is_enabled(
                        __CALLSITE.metadata(),
                        interest,
                    )
            };
        if enabled {
            (|value_set: ::tracing::field::ValueSet| {
                let meta = __CALLSITE.metadata();
                ::tracing::Event::dispatch(meta, &value_set);
                if match ::tracing::Level::INFO {
                    ::tracing::Level::ERROR => ::tracing::log::Level::Error,
                    ::tracing::Level::WARN => ::tracing::log::Level::Warn,
                    ::tracing::Level::INFO => ::tracing::log::Level::Info,
                    ::tracing::Level::DEBUG => ::tracing::log::Level::Debug,
                    _ => ::tracing::log::Level::Trace,
                } <= ::tracing::log::STATIC_MAX_LEVEL
                {
                    #[allow(unused_braces)]
                    {
                        use ::tracing::log;
                        let level = match ::tracing::Level::INFO {
                            ::tracing::Level::ERROR => ::tracing::log::Level::Error,
                            ::tracing::Level::WARN => ::tracing::log::Level::Warn,
                            ::tracing::Level::INFO => ::tracing::log::Level::Info,
                            ::tracing::Level::DEBUG => ::tracing::log::Level::Debug,
                            _ => ::tracing::log::Level::Trace,
                        };
                        if level <= log::max_level() {
                            let meta = __CALLSITE.metadata();
                            let log_meta = log::Metadata::builder()
                                .level(level)
                                .target(meta.target())
                                .build();
                            let logger = log::logger();
                            if logger.enabled(&log_meta) {
                                ::tracing::__macro_support::__tracing_log(
                                    meta,
                                    logger,
                                    log_meta,
                                    &value_set,
                                )
                            }
                        }
                    }
                } else {
                    {}
                };
            })({
                #[allow(unused_imports)]
                use ::tracing::field::{debug, display, Value};
                let mut iter = __CALLSITE.metadata().fields().iter();
                __CALLSITE
                    .metadata()
                    .fields()
                    .value_set(
                        &[
                            (
                                &::tracing::__macro_support::Iterator::next(&mut iter)
                                    .expect("FieldSet corrupted (this is a bug)"),
                                ::tracing::__macro_support::Option::Some(
                                    &format_args!("lolwhat") as &dyn Value,
                                ),
                            ),
                        ],
                    )
            });
        } else {
            if match ::tracing::Level::INFO {
                ::tracing::Level::ERROR => ::tracing::log::Level::Error,
                ::tracing::Level::WARN => ::tracing::log::Level::Warn,
                ::tracing::Level::INFO => ::tracing::log::Level::Info,
                ::tracing::Level::DEBUG => ::tracing::log::Level::Debug,
                _ => ::tracing::log::Level::Trace,
            } <= ::tracing::log::STATIC_MAX_LEVEL
            {
                #[allow(unused_braces)]
                {
                    use ::tracing::log;
                    let level = match ::tracing::Level::INFO {
                        ::tracing::Level::ERROR => ::tracing::log::Level::Error,
                        ::tracing::Level::WARN => ::tracing::log::Level::Warn,
                        ::tracing::Level::INFO => ::tracing::log::Level::Info,
                        ::tracing::Level::DEBUG => ::tracing::log::Level::Debug,
                        _ => ::tracing::log::Level::Trace,
                    };
                    if level <= log::max_level() {
                        let meta = __CALLSITE.metadata();
                        let log_meta = log::Metadata::builder()
                            .level(level)
                            .target(meta.target())
                            .build();
                        let logger = log::logger();
                        if logger.enabled(&log_meta) {
                            ::tracing::__macro_support::__tracing_log(
                                meta,
                                logger,
                                log_meta,
                                &{
                                    #[allow(unused_imports)]
                                    use ::tracing::field::{debug, display, Value};
                                    let mut iter = __CALLSITE.metadata().fields().iter();
                                    __CALLSITE
                                        .metadata()
                                        .fields()
                                        .value_set(
                                            &[
                                                (
                                                    &::tracing::__macro_support::Iterator::next(&mut iter)
                                                        .expect("FieldSet corrupted (this is a bug)"),
                                                    ::tracing::__macro_support::Option::Some(
                                                        &format_args!("lolwhat") as &dyn Value,
                                                    ),
                                                ),
                                            ],
                                        )
                                },
                            )
                        }
                    }
                }
            } else {
                {}
            };
        }
    };
    let mut terminal = TerminalSurface::default();
    terminal.draw(etch_splash_screen)?;
    Ok(ExitCode::SUCCESS)
}
fn etch_splash_screen(_scaffold: &mut Scaffold) -> Result<(), ()> {
    {
        use ::slate::style::StyleSheet;
        use ::slate::style::primitive::*;
        use ::slate::style::primitive::Unit::*;
        use ::slate::style::property::*;
        use ::slate::event::EventPin::*;
        #[allow(unnecessary_braces)] #[allow(unused_braces)] #[allow(unused_parens)]
        move |scaffold| {
            let _00000 = scaffold
                .add({ Container::default().with_alt("Splash Screen") })?
                .with_children(|scaffold| {
                    let _00001 = scaffold
                        .add({ Text::default().with_video(2) })?
                        .with_children(|scaffold| { Ok(()) })?
                        .build();
                    Ok(())
                })?
                .build();
            Ok(())
        }
    }
}
fn etch_drawing_surface(_scaffold: &mut Scaffold) -> Result<(), ()> {
    {
        use ::slate::style::StyleSheet;
        use ::slate::style::primitive::*;
        use ::slate::style::primitive::Unit::*;
        use ::slate::style::property::*;
        use ::slate::event::EventPin::*;
        #[allow(unnecessary_braces)] #[allow(unused_braces)] #[allow(unused_parens)]
        move |scaffold| {
            let _00002 = scaffold
                .add({ Container::default().with_alt("Drawing Surface") })?
                .with_children(|scaffold| {
                    let _00003 = scaffold
                        .add({ Text::default().with_video(2) })?
                        .with_children(|scaffold| { Ok(()) })?
                        .build();
                    Ok(())
                })?
                .build();
            Ok(())
        }
    }
}
enum EtchError {
    #[msg("unknown error: {0}")]
    Unknown(anyhow::Error),
}
impl core::error::Error for EtchError {}
impl core::fmt::Display for EtchError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.write_fmt(
            format_args!(
                "{0}", match self { EtchError::Unknown(x0) =>
                ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("unknown error: {0}", x0)) }), }
            ),
        )
    }
}
impl core::fmt::Debug for EtchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(
            format_args!(
                "{0}", match self { EtchError::Unknown(x0) =>
                ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("unknown error: {0}", x0)) }), }
            ),
        )
    }
}
#[allow(unreachable_code)]
#[automatically_derived]
impl derive_more::core::convert::From<(anyhow::Error)> for EtchError {
    #[inline]
    fn from(value: (anyhow::Error)) -> Self {
        EtchError::Unknown(value)
    }
}
