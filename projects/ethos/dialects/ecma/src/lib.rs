//! The ECMAScript/TypeScript dialect: parses source into an AST, prints an AST back to
//! whitespace-and-comment-preserving source. Nothing here executes code — that's `ethos-deno`'s
//! job, paired with this dialect by `ethos-cli`, not by any dependency between the two crates.

use std::sync::Arc;

use swc_core::common::comments::SingleThreadedComments;
use swc_core::common::FileName;
use swc_core::common::FilePathMapping;
use swc_core::common::SourceMap;
use swc_core::common::input::SourceFileInput;
use swc_core::ecma::ast::EsVersion;
use swc_core::ecma::ast::Module;
use swc_core::ecma::codegen::Config;
use swc_core::ecma::codegen::Emitter;
use swc_core::ecma::codegen::text_writer::JsWriter;
use swc_core::ecma::parser::lexer::Lexer;
use swc_core::ecma::parser::Parser;
use swc_core::ecma::parser::Syntax;
use swc_core::ecma::parser::TsSyntax;

/// A parsed module plus everything `print` needs to reproduce its original whitespace and
/// comments — swc's emitter works from byte positions recorded against the `SourceMap` the module
/// was parsed with, not from the AST alone.
pub struct EcmaAst {
    module: Module,
    comments: SingleThreadedComments,
    source_map: Arc<SourceMap>,
}

#[derive(Debug)]
pub enum EcmaDialectError {
    Parse(swc_core::ecma::parser::error::Error),
    Emit(std::io::Error),
}

impl std::fmt::Display for EcmaDialectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EcmaDialectError::Parse(error) => write!(f, "parse error: {error:?}"),
            EcmaDialectError::Emit(error) => write!(f, "emit error: {error}"),
        }
    }
}

impl std::error::Error for EcmaDialectError {}

#[derive(Default)]
pub struct EcmaDialect;

impl ethos_core::Dialect for EcmaDialect {
    type Ast = EcmaAst;
    type Error = EcmaDialectError;

    fn parse(&self, source: &str) -> Result<Self::Ast, Self::Error> {
        let source_map = Arc::new(SourceMap::new(FilePathMapping::empty()));
        let source_file = source_map.new_source_file(Arc::new(FileName::Anon), source.to_string());

        let syntax = Syntax::Typescript(TsSyntax {
            tsx: true,
            decorators: true,
            ..Default::default()
        });

        let mut comments = SingleThreadedComments::default();
        let lexer = Lexer::new(syntax, EsVersion::latest(), SourceFileInput::from(&*source_file), Some(&mut comments));
        let mut parser = Parser::new_from(lexer);

        let module = parser.parse_module().map_err(EcmaDialectError::Parse)?;

        Ok(EcmaAst { module, comments, source_map })
    }

    fn print(&self, ast: &Self::Ast) -> Result<String, Self::Error> {
        let mut output = Vec::new();

        let mut emitter = Emitter {
            cm: ast.source_map.clone(),
            cfg: Config::default(),
            comments: Some(&ast.comments),
            wr: JsWriter::new(ast.source_map.clone(), "\n", &mut output, None),
        };

        emitter.emit_module(&ast.module).map_err(EcmaDialectError::Emit)?;

        Ok(String::from_utf8_lossy(&output).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethos_core::Dialect;

    #[test]
    fn round_trips_source_through_parse_and_print() {
        let dialect = EcmaDialect;
        let ast = dialect.parse("const x = 1;").expect("parse should succeed");
        let printed = dialect.print(&ast).expect("print should succeed");
        assert!(printed.contains("const x = 1;"));
    }
}
