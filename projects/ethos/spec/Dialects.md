# Dialects and runtimes

A **dialect** parses source into an AST and prints an AST back to whitespace-and-comment-preserving source. Nothing else — no execution, no FFI, no opinion on what runs the code it parses. A **runtime** executes code. Nothing else — no opinion on which dialect produced that code. Both are real traits in `packages/core` (`ethos_core::Dialect`, `ethos_core::Runtime`).

`ethos-cli` pairs a dialect with a runtime per file, by extension — `.js`/`.ts` with `ethos-deno`, `.lua` with `ethos-lua` — not by any dependency between the two crates. That's the whole point of the split: a dialect and a runtime never need to know about each other, so a new dialect can pair with a new runtime (a future compiled-language dialect with LLVM codegen, an interpreter-only backend for a WebGL-hosted host, whatever) without touching anything that already exists.

## Current dialects

| Dialect | Crate | Parses/prints via | Pairs with |
|---|---|---|---|
| ECMAScript/TypeScript | `ethos-ecma` | `swc_core` | `ethos-deno` (V8/`deno_core`) |
| Lua | `ethos-lua` | — (`mlua` runs it directly, no separate AST step) | itself (`mlua`) |
| C | `ethos-c` | hand-rolled lexer/parser | itself (a hand-rolled tree-walking interpreter) |

`ethos-lua` and `ethos-c` are the pragmatic middle step described below — real parsing/execution, not LLVM codegen.

## The original, larger vision (not built yet)

Ported from `legacy/examples/franken/README.md` (2026-08-14) — the clearest surviving statement of Ethos's founding architecture, from before the current `Dialect`/`Runtime` trait split existed. Kept verbatim because it's still the long-term target, just not what's implemented today:

> In Ethos, a Dialect is a plugin for Ethos which provides parsing and compilation utilities for a single source input. A Dialect could build modules from code, images, Excel docs, or any other expressive input.
>
> The primary goal of Ethos is to compile various input sources into LLVM modules which can then be executed directly or embedded in other software from a C-like interface.
>
> For example, given an Excel file and a JavaScript file, I could import the excel file via JavaScript import, an Excel dialect could parse an .xls file into a simple LLVM module table who's values can be accessed, or who's functions could be called.

`dialects/llvm` (not a workspace member — needs a version-matched LLVM install for `inkwell`, not set up in every dev environment) is the reference starting point for this, whenever it's picked back up. Until then, a dialect pairs with a real interpreter/engine `Runtime` instead of compiling to LLVM IR — `ethos-deno`/`ethos-lua` above are that pragmatic version of the same "a dialect plugs into Ethos" idea, just executing directly instead of compiling first.
