//! The two traits every dialect and runtime in Ethos is built against.
//!
//! A dialect knows only syntax: parse source into an AST, print an AST back to source. A runtime
//! knows only execution: run source, return what it produced. Neither knows about the other —
//! `ethos-cli` pairs a dialect with a runtime per file, by extension, not by any coupling between
//! the two crates themselves. This is what lets unrelated backends (V8 for JS-like dialects,
//! `mlua` for Lua, eventually LLVM-backed compilation for others) sit behind the same interface.

/// Parses source text into an AST and prints it back out, preserving whitespace and comments.
pub trait Dialect {
    type Ast;
    type Error: std::error::Error;

    fn parse(&self, source: &str) -> Result<Self::Ast, Self::Error>;
    fn print(&self, ast: &Self::Ast) -> Result<String, Self::Error>;
}

/// Executes source text and returns whatever it produced (return value, captured output, or
/// both, at the implementor's discretion).
pub trait Runtime {
    type Error: std::error::Error;

    fn execute(&mut self, source: &str) -> Result<String, Self::Error>;
}
