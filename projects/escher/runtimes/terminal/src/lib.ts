const TARGET_DIR = ".cargo/target/debug";
const LIB_ARTIFACT = "escher_terminal.dll";

export const dylib = Deno.dlopen(`${TARGET_DIR}/${LIB_ARTIFACT}`, {
    init: { parameters: [], result: "void" } as const,
});

export function init() {
    return dylib.symbols.init();
}
