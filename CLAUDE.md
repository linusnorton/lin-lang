# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

`lin-lang` is the reference implementation of **Lin**, a small expression-based language built around strict JSON data, structural typing, first-argument function application (dot syntax), destructuring, pattern matching, opaque iterator/runtime types, and value-based error handling. The full language design is in `docs/SPECIFICATION.md`.

The project has two backends:

- **`lin-eval`** — a tree-walking interpreter (`lin run`). The v0 backend; types are parsed and stored in the AST but not enforced at runtime (see ADR-001 in `docs/DECISIONS.md`).
- **`lin-codegen`** — an LLVM native-code compiler (`lin build`). Goes through a full type-checking pass (`lin-check`), lowers to flat 3-address IR (`lin-ir`), then to LLVM IR via `inkwell`. Links with a small Rust static library (`lin-runtime`) to produce a standalone binary.

## Build / run / test

```bash
cargo build --workspace
cargo test --workspace                          # runs all unit + integration tests
cargo test -p lin-eval test_hello_world         # run a single test
cargo run -p lin -- examples/hello.lin          # interpret a .lin program (lin run)
cargo run -p lin -- run examples/hello.lin      # same, explicit subcommand
cargo run -p lin -- build examples/hello.lin -o hello  # compile to native binary
cargo run -p lin -- check examples/hello.lin    # type check only
cargo run -p lin -- -                           # read source from stdin
```

Environment variables for `lin build`:
- `LIN_EMIT_IR=1` — write the LLVM IR (`.ll`) alongside the binary
- `LIN_NO_OPT=1` — skip LLVM optimisation passes (faster builds, slower output)

There is no CI config or formatter wired up yet. There is no `cargo` available at the system shell at the time of writing — assume the user runs commands themselves.

## Workspace layout

Cargo workspace with ten crates (`crates/`):

- **`lin-common`** — shared `Span`, `Diagnostic`, `Interner`, edit-distance helpers. No dependencies on other crates.
- **`lin-lex`** — lexer with indentation tracking. Produces `Token` stream with synthetic `Indent`/`Dedent`/`Newline` tokens.
- **`lin-parse`** — parser, surface AST (`Module`, `Stmt`, `Expr`, `Pattern`, `TypeExpr`). Includes parser error recovery and "did you mean" diagnostics.
- **`lin-check`** — type checker. Consumes the surface AST; produces `TypedModule` (typed IR). Handles bidirectional inference, structural typing, union narrowing, exhaustiveness checking, TypeVar zonking, and numeric widening. Emits `Diagnostic` values with Ariadne-style multi-span rendering.
- **`lin-ir`** — flat 3-address IR (`LinIR`) sitting between `TypedExpr` and LLVM. Contains: IR data types (`ir.rs`), the `TypedModule → LinModule` lowering pass (`lower.rs`), backwards-dataflow liveness analysis (`liveness.rs`), and the Perceus-inspired RC elision pass (`rc_elide.rs`).
- **`lin-codegen`** — LLVM backend via `inkwell`. Compiles `TypedModule` directly to LLVM IR today; `lin-ir` is available as an optional pre-pass. Handles functions, closures, objects, arrays, strings, union tagged dispatch, pattern matching, TCO, and unboxed scalar arrays.
- **`lin-runtime`** — small static library linked into every compiled binary. Provides refcounted strings/arrays/objects, intrinsics (`lin_print`, `lin_string_concat`, etc.), and flat scalar array variants (`lin_flat_array_alloc_i32`, etc.).
- **`lin-compile`** — orchestrates the full compilation pipeline: source → lex → parse → type check → codegen → link. Includes a module cache (`.lin-cache/<sha256>.typed`) and module signature files (`.lin-cache/<sha256>.sig`) to skip re-checking unchanged imports.
- **`lin-eval`** — tree-walking interpreter (the `lin run` backend). Owns `Value`, `Env`, `Interpreter`.
- **`lin`** — CLI binary. Dispatches `run`, `build`, `check` subcommands.
- **`lin-lsp`** — language server (in progress).

Stdlib lives in `stdlib/*.lin` and is loaded via `include_str!` in both `lin-eval` and `lin-compile`.

## Pipeline shapes

**Interpreter (`lin run`)**:
```
source (.lin) → Lexer → Tokens → Parser → AST → Interpreter::eval → Value
```

Everything happens in `lin-eval::Interpreter`:

1. `Interpreter::new()` calls `register_intrinsics()` (native Rust functions like `print`, `length`, `toString`, `__stringTrim`, `iter`, `for`, `push`, ...) then `register_stdlib_sources()` (embeds the `stdlib/*.lin` files via `include_str!`) then `preload_stdlib()` (loads `std/iter` and `std/array` into the global env so `range`, `map`, `filter`, etc. are globally available).
2. `run_file(path)` sets `base_path` (used for resolving user imports) and calls `run(source)`.
3. `run(source)` lexes, parses, then evaluates statements top-to-bottom in `global_env`.

The interpreter is a single ~1400-line file (`crates/lin-eval/src/interpreter.rs`). Most language features live there.

**Compiler (`lin build`)**:
```
source (.lin)
  → Lexer → Tokens → Parser → AST
  → lin-check: type checker → TypedModule
  → lin-ir (optional): TypedModule → LinModule (flat 3-address IR) → RC elision pass
  → lin-codegen: TypedModule → LLVM IR (via inkwell)
  → LLVM optimisation passes (default: O2)
  → emit .o object file
  → cc link with lin-runtime.a → native binary
```

Imports are resolved recursively before the main module is checked. Each imported module is type-checked once and cached by source hash in `.lin-cache/`. If the cache hit is valid, the `TypedModule` is deserialised instead of re-checked. A separate `ModuleSignature` (`.sig` file) records just the exported name→type map; dependents only need that to verify their own usage, not the full IR.

## Key design choices to be aware of

These are non-obvious and easy to break. Full rationale lives in `docs/DECISIONS.md` — read it before making structural changes.

- **Indentation lexing is suppressed inside `{ }`, `( )`, `[ ]`.** This lets JSON object literals span lines without triggering block parsing. Don't add INDENT/DEDENT logic inside delimiter-balanced spans (ADR-004, ADR-017).
- **String interpolation is one compound token** (`InterpString(Vec<InterpPart>)`) whose `Expr` parts each carry their own sub-token-stream. The parser recurses into those sub-streams (ADR-005).
- **Dot-chaining across newlines uses save/restore lookahead** in the parser's postfix loop. Don't aggressively skip newlines — it breaks block structure (ADR-006). After a `Dedent`, postfix `[` and `(` are suppressed but `.` is allowed (ADR-011).
- **Bare-identifier lambdas (`x => x * 2`) are only recognised in argument position.** `is_bare_lambda()` looks ahead from inside argument parsing (ADR-007).
- **`val` whose RHS is a function literal is forward-declared via mutable cells** before evaluation, so mutual recursion works between top-level functions (ADR-015). Non-function `val` cannot self-reference (spec §7.3).
- **Top-level statements clone `global_env`, evaluate, then write back.** This is to dodge a borrow checker conflict between `&mut self` (needed for `call_value`) and `&mut self.global_env` (ADR-008). The clone is O(n) bindings — acceptable for v0.
- **TCO uses a `TailResult` trampoline.** `eval_tail_expr` recognises direct self-recursive calls in tail position (function body, if branches, block tails, match arm bodies) and returns `TailCall(args)` instead of recursing. Only `call_function` loops on it. Mutual TCO is not implemented (ADR-012, spec §27.3).
- **`var` is captured by reference via shared `Rc<RefCell<Value>>` cells.** Two closures over the same `var` see the same storage (spec §27.2, ADR-015).
- **Bracket access is safe by default.** Missing object key → `Null`; `Null` propagates through chains; array OOB is a runtime error (spec §6.1).
- **Stdlib split: `for` and `iter` are Rust intrinsics, everything else is .lin.** `range`, `iterOf`, `map`, `filter`, `reduce` live in `stdlib/{iter,array}.lin` and are preloaded as globals (ADR-002). String functions are .lin wrappers around `__stringFoo` Rust intrinsics (ADR-009).
- **Inline blocks inside parentheses.** Lambdas like `x => val y = x*2; y` passed to `.for(...)` have no INDENT/DEDENT (suppressed by ADR-004). `parse_function_body` detects `val`/`var` as the multi-statement-body signal (ADR-014).
- **Imports: `std/...` resolves into the embedded stdlib sources; everything else is resolved relative to the importing file's directory with `.lin` appended** (ADR-016). Module init is lazy; cycles within a single init chain are a runtime error.

## Adding a language feature

The typical path:

1. **Tokens** — add `TokenKind` variants in `lin-lex/src/token.rs`, lex them in `lin-lex/src/lexer.rs`. Remember the indentation suppression invariants for new delimiters.
2. **AST** — add `Expr`/`Stmt`/`Pattern`/`TypeExpr` variants in `lin-parse/src/ast.rs`. Each variant carries its own `Span`. Add a branch in `Expr::span()`.
3. **Parser** — wire into `lin-parse/src/parser.rs`. For postfix operators, mind the DEDENT suppression rule (ADR-011). For continuation-line constructs, use the `skip_continuation_newline` pattern (ADR-013).
4. **Interpreter** — add a match arm in `eval_expr_in_env` (and `eval_tail_expr` if it can appear in tail position). Native helpers go through `define_native(name, arity, |args| ...)` in `register_intrinsics`.
5. **Tests** — add a case to `crates/lin-eval/tests/integration.rs` and, ideally, an end-to-end fixture in `examples/`.

There is no desugaring pass — the interpreter consumes the surface AST directly. Things the spec describes as desugarings (`x.f(y)` → `f(x, y)`, destructuring → primitive bindings) are implemented inline in the evaluator.

## Adding a stdlib function

Make sure it is included in the `docs/STDLIB.md` documentation.

## Where things live by topic

- **Operator precedence** — `parse_or_expr` → `parse_and_expr` → `parse_comparison` → ... in `lin-parse/src/parser.rs`. Mirror the spec §24.2 ladder when changing.
- **Iterator semantics** — `IteratorValue` struct in `lin-eval/src/value.rs`; the `for` intrinsic and `iter` constructor in `register_intrinsics`. Per spec §17.6, do not model iterators as JSON-shaped objects.
- **Equality** — `Value::deep_eq` in `lin-eval/src/value.rs`. Objects are order-independent; arrays are ordered; cross-numeric (`Int == Float`) compares by value.
- **Display / `toString`** — `Value::to_display_string` and `to_json_string` in the same file. Used by string interpolation.

## Reading order for a new contributor

1. `docs/SPECIFICATION.md` — what the language is meant to be.
2. `docs/STDLIB.md` — specification of the stdlib.
3. `docs/DECISIONS.md` — every non-obvious implementation choice and why. **Read this before touching the lexer or parser.**
4. `docs/TODO.md` — milestone plan. Note the gap: §3 specifies type checking, but v0 has none.
5. `crates/lin-eval/src/interpreter.rs` — the engine.
6. `examples/*.lin` and `crates/lin-eval/tests/integration.rs` — what currently works.
