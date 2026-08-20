//! A quick-and-dirty C-like dialect for Ethos: a small hand-rolled lexer/parser/interpreter for
//! a reduced subset (`int` variables, arithmetic, comparisons, `if`/`else`, `while`, and a
//! `printf` builtin that only understands `%d`) — not a conformant C implementation.
//!
//! The backup this was pulled forward from (`examples/franken/README.md`, ported alongside this
//! crate) describes Ethos's actual target architecture: dialects parse *and compile* their
//! source into LLVM modules via `inkwell`, executable directly or embeddable from a C-like
//! interface. That's the real goal, but wiring up `inkwell` needs a version-matched LLVM install
//! (this machine has `clang` but no `llvm-config`), so it's out of scope here. This dialect is
//! the pragmatic middle step: prove out "parse a source string, run it, capture its output" as a
//! dialect boundary, the same shape `dialects/ecma` uses around a real embedded JS engine.

mod interp;
mod lexer;
mod parser;

pub use interp::Interpreter;
pub use parser::Parser;

/// Parses and runs a chunk of the dialect's source, returning every line printed via `printf`.
pub fn run(source: &str) -> Vec<String> {
    let tokens = lexer::lex(source);
    let program = Parser::new(tokens).parse_program();
    Interpreter::new().run(&program).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_and_printf() {
        let output = run(
            r#"
            int a = 3;
            int b = 4;
            printf("%d\n", a + b * 2);
            "#,
        );
        assert_eq!(output, vec!["11\n".to_string()]);
    }

    #[test]
    fn while_loop_and_if() {
        let output = run(
            r#"
            int i = 0;
            int sum = 0;
            while (i < 5) {
                if (i % 2 == 0) {
                    sum = sum + i;
                }
                i = i + 1;
            }
            printf("sum=%d\n", sum);
            "#,
        );
        assert_eq!(output, vec!["sum=6\n".to_string()]);
    }
}
