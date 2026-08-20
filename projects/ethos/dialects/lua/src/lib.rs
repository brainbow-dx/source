//! A quick-and-dirty Lua dialect for Ethos: real Lua 5.4 via `mlua` (vendored, so no system Lua
//! install is required), not a hand-rolled parser. This is the pragmatic end of the spectrum —
//! `franken`'s design doc (`examples/franken/README.md`, ported from the pre-refactor backup)
//! describes Ethos dialects as eventually compiling every source into LLVM modules; this dialect
//! doesn't do that yet. It just proves out "parse + run a source string, capture its output" as
//! a dialect boundary, the same way `dialects/ecma` embeds a real JS engine rather than writing
//! one from scratch.

use std::sync::Arc;
use std::sync::Mutex;

use mlua::Lua;
use mlua::MultiValue;

#[derive(Debug)]
pub enum LuaError {
    Runtime(mlua::Error),
}

impl std::fmt::Display for LuaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LuaError::Runtime(error) => write!(f, "lua error: {error}"),
        }
    }
}

impl std::error::Error for LuaError {}

impl From<mlua::Error> for LuaError {
    fn from(error: mlua::Error) -> Self {
        LuaError::Runtime(error)
    }
}

/// A single Lua VM instance with `print` wired to an in-memory buffer instead of stdout, so
/// output can be captured and inspected instead of just watched fly by in a terminal.
pub struct LuaRuntime {
    lua: Lua,
    output: Arc<Mutex<Vec<String>>>,
}

impl LuaRuntime {
    pub fn new() -> Result<Self, LuaError> {
        let lua = Lua::new();
        let output = Arc::new(Mutex::new(Vec::new()));

        let output_for_print = output.clone();
        let print = lua.create_function(move |_, args: MultiValue| {
            let line = args.iter().map(|value| value.to_string().unwrap_or_default()).collect::<Vec<_>>().join("\t");
            output_for_print.lock().unwrap().push(line);
            Ok(())
        })?;
        lua.globals().set("print", print)?;

        Ok(LuaRuntime { lua, output })
    }

    /// Runs a chunk of Lua source and returns everything it `print`ed, in order.
    pub fn eval(&self, source: &str) -> Result<Vec<String>, LuaError> {
        self.output.lock().unwrap().clear();
        self.lua.load(source).exec()?;
        Ok(self.output.lock().unwrap().clone())
    }
}

impl ethos_core::Runtime for LuaRuntime {
    type Error = LuaError;

    /// Joins every line `print`ed while running `source` — the same "capture printed output"
    /// contract `ethos_deno::worker::DenoRuntime` implements for JS, so `ethos-cli` can dispatch
    /// to either backend uniformly.
    fn execute(&mut self, source: &str) -> Result<String, Self::Error> {
        Ok(self.eval(source)?.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethos_core::Runtime;

    #[test]
    fn runtime_trait_executes_and_joins_output() {
        let mut runtime = LuaRuntime::new().expect("lua runtime");
        let output = runtime.execute("print('a')\nprint('b')").expect("execute");
        assert_eq!(output, "a\nb");
    }

    #[test]
    fn captures_print_output() {
        let runtime = LuaRuntime::new().expect("lua runtime");
        let output = runtime.eval("print('hello from lua')").expect("eval");
        assert_eq!(output, vec!["hello from lua".to_string()]);
    }

    #[test]
    fn runs_real_lua_control_flow() {
        let runtime = LuaRuntime::new().expect("lua runtime");
        let output = runtime
            .eval(
                r#"
                local function fib(n)
                    if n < 2 then return n end
                    return fib(n - 1) + fib(n - 2)
                end

                for i = 0, 6 do
                    print(i, fib(i))
                end
                "#,
            )
            .expect("eval");

        assert_eq!(output.len(), 7);
        assert_eq!(output[6], "6\t8");
    }
}
