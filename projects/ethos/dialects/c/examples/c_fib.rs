//! Runs an iterative Fibonacci program written in `ethos-c`'s reduced C-like subset (no
//! user-defined functions yet — see `src/lib.rs` — so this is written iteratively) and prints
//! whatever it `printf`s.

fn main() {
    let source = r#"
        int a = 0;
        int b = 1;
        int i = 0;
        while (i < 10) {
            printf("fib(%d) = %d\n", i, a);
            int next = a + b;
            a = b;
            b = next;
            i = i + 1;
        }
    "#;

    for line in ethos_c::run(source) {
        print!("{line}");
    }
}
