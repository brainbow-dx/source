#![allow(unused)]
#![feature(allocator_api)]

extern crate alloc;

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use clap::Parser as ArgParser;

use eyre::Result;

use derive_more::Deref;
use derive_more::DerefMut;
use derive_more::Display;

use swc_core::common::BytePos;
use swc_core::common::FilePathMapping;
use swc_core::common::SourceMap;
use swc_core::common::comments::Comment;
use swc_core::common::comments::CommentKind;
use swc_core::common::comments::Comments;
use swc_core::common::comments::SingleThreadedComments;
use swc_core::common::input::SourceFileInput;
use swc_core::ecma::ast::EsVersion;
use swc_core::ecma::ast::Expr;
use swc_core::ecma::ast::Ident;
use swc_core::ecma::ast::Module;
use swc_core::ecma::ast::ModuleItem;
use swc_core::ecma::codegen::Config;
use swc_core::ecma::codegen::Emitter;
use swc_core::ecma::codegen::text_writer::JsWriter;
use swc_core::ecma::parser::Parser;
use swc_core::ecma::parser::Syntax;
use swc_core::ecma::parser::TsSyntax;
use swc_core::ecma::parser::lexer::Lexer;

#[derive(ArgParser, Debug)]
#[command(version, about, long_about=None)]
pub struct Args {
    #[arg(short, long, default_value="trace")]
    log_filter: String,

    #[arg(short, long)]
    inspect: bool,

    #[arg(default_value="./examples/template.tsx")]
    entrypoint: PathBuf,
}

//---
/// Usage: ./scripts/instruments path/to/input/file
fn main() -> Result<ExitCode> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&args.log_filter)
        .with_level(true)
        .without_time()
        .with_target(true)
        .with_ansi(true)
        .init();

    // TODO: Put these values in a struct ..
    let sourcemap = Arc::new(SourceMap::new(FilePathMapping::default()));
    let mut entrypoint = EcmaModule::try_parse(sourcemap.clone(), &args.entrypoint)?;

    tracing::debug!("Parsed '{:?}' in {:?}", entrypoint.specifier(), entrypoint.elapsed());

    // TODO: Move this logic to EcmaModule ..
    #[cfg(all(feature = "dev", feature = "verbose"))]
    for item in entrypoint.body.iter_mut() {
        tracing::debug!("Statement: {:?}", item)
    }

    // TODO: Move this to parse_`ethos_ecma::parse_comments(..)` or whatever ..
    let comments = ParsedComments::from(entrypoint.comments());

    if args.inspect {
        comments.trace();
    }

    let new_ident = Ident::from("someNewVariable: number");
    let new_assignment_stmt = swc_core::quote!(
        "const $name = $val" as Stmt,
        name: Ident = new_ident.clone(),
        val: Expr = swc_core::quote!("4" as Expr),
    );
    let new_call_stmt = swc_core::quote!("console.log('TODO:', $new_ident);" as Stmt, new_ident = new_ident.clone(),);

    entrypoint.body.push(new_assignment_stmt.into());
    entrypoint.body.push(new_call_stmt.into());

    let mut output_source = Vec::new();
    let mut emitter = Emitter {
        cm: sourcemap.clone(),
        cfg: Config::default().with_minify(false),
        comments: Some(&entrypoint.comments()),
        wr: JsWriter::new(sourcemap.clone(), "\n", &mut output_source, None),
    };

    if let Err(error) = emitter.emit_module(&entrypoint) {
        eprintln!("Failed to emit module: {:}", error);
    }

    tracing::debug!("Rendered Output:\n{:}", String::from_utf8_lossy(&output_source));

    Ok(ExitCode::SUCCESS)
}

//---
#[derive(Default, Deref, DerefMut)]
pub struct ParsedModule<C> {
    #[deref]
    #[deref_mut]
    module: Module,

    comments: C,

    elapsed: Duration,
}

impl<C> ParsedModule<C> {
    pub fn new(module: Module, comments: C, elapsed: Duration) -> Self {
        ParsedModule {
            module,
            comments,
            elapsed,
        }
    }
}

impl<C> ParsedModule<C> {
    pub fn comments(&self) -> &C {
        &self.comments
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

pub struct ParsedComments<'comments, C> {
    comments: &'comments C,
}

impl<'comments, C> From<&'comments C> for ParsedComments<'comments, C> {
    fn from(comments: &'comments C) -> Self {
        ParsedComments {
            comments,
        }
    }
}

impl ParsedComments<'_, SingleThreadedComments> {
    pub fn trace(&self) {
        let (leading, trailing) = self.comments.borrow_all();

        for (position, comments) in leading.iter().chain(trailing.iter()) {
            tracing::debug!("Found Comments @ {:?}", position);

            if self.comments.has_flag(*position, "asdf") {
                tracing::debug!("Flag: asdf");
            }

            for comment in comments.iter() {
                match comment.kind {
                    CommentKind::Block => {
                        tracing::debug!("DocBlock:\n{:#?}", comment);
                    }
                    CommentKind::Line => {
                        tracing::debug!("{:?} //{:}", comment.span, comment.text);
                    }
                }
            }
        }
    }
}

#[derive(Display, Deref, DerefMut)]
#[display("TODO: {specifier}")]
pub struct EcmaModule<S, C = SingleThreadedComments> {
    specifier: S,

    #[deref]
    #[deref_mut]
    parsed: ParsedModule<C>,
}

impl<S, C: Default> EcmaModule<S, C> {
    pub fn new(specifier: S) -> Self {
        EcmaModule {
            specifier,
            parsed: ParsedModule::default(),
        }
    }

    pub fn with_parsed(mut self, parsed: ParsedModule<C>) -> Self {
        self.parsed = parsed;
        self // etc..
    }
}

impl<S> EcmaModule<S> {
    pub fn specifier(&self) -> &S {
        &self.specifier
    }
}

impl<S: AsRef<Path>> EcmaModule<S> {
    pub fn try_parse(sourcemap: Arc<SourceMap>, entry_module: S) -> Result<Self, TemplateError> {
        let entry_source = sourcemap.load_file(entry_module.as_ref()).map_err(TemplateError::LoadFailed)?;
        let entry_source = SourceFileInput::from(entry_source.as_ref());

        let es_version = EsVersion::latest();
        let syntax = Syntax::Typescript(TsSyntax {
            dts: true,
            tsx: true,
            decorators: true,
            no_early_errors: false,
            disallow_ambiguous_jsx_like: false,
        });
        let mut comments = SingleThreadedComments::default();

        let lexer = Lexer::new(syntax, es_version, entry_source, Some(&mut comments));
        let mut parser = Parser::new_from(lexer);
        let parse_duration = Instant::now();

        let parsed_module = parser.parse_module().map_err(TemplateError::ParseFailed)?;

        Ok(EcmaModule::new(entry_module).with_parsed(ParsedModule::new(
            parsed_module,
            comments,
            parse_duration.elapsed(),
        )))
    }
}

#[derive(oops::Error)]
pub enum TemplateError {
    #[msg("load from file error: {0}")]
    LoadFailed(std::io::Error),

    #[msg("parse error: {0:?}")]
    ParseFailed(swc_core::ecma::parser::error::Error),
}
