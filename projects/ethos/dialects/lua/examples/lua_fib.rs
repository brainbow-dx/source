//! Runs a real Lua 5.4 program (recursive Fibonacci, actual `local function`/recursion; this
//! dialect embeds `mlua`, not a hand-rolled subset) and prints whatever it `print`s.

use ethos_lua::LuaRuntime;

fn main() {
    let runtime = LuaRuntime::new().expect("lua runtime");

    let output = runtime
        .eval(
            r#"
            local function fib(n)
                if n < 2 then return n end
                return fib(n - 1) + fib(n - 2)
            end

            for i = 0, 9 do
                print(string.format("fib(%d) = %d", i, fib(i)))
            end
            "#,
        )
        .expect("eval");

    for line in output {
        println!("{line}");
    }
}
