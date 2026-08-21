//! Parses real TypeScript into an AST via `EcmaDialect` and prints it back out — the dialect's
//! whole job, syntax only. Nothing here executes; pairing this dialect with a runtime is
//! `ethos-cli`'s job (see `ethos-deno`'s own `eval` example for the execution half).

use ethos_core::Dialect;
use ethos_ecma::EcmaDialect;

fn main() {
    let dialect = EcmaDialect;

    let source = r#"
        interface Point {
            x: number;
            y: number;
        }

        function distance(a: Point, b: Point): number {
            // Real comments and formatting survive the round trip.
            return Math.sqrt((a.x - b.x) ** 2 + (a.y - b.y) ** 2);
        }
    "#;

    let ast = dialect.parse(source).expect("parse");
    let printed = dialect.print(&ast).expect("print");

    println!("{printed}");
}
