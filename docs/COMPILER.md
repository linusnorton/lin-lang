# Lin Compiler Architecture

This document describes the native-code compilation pipeline: `lin build`. It covers the design of each stage, the data structures passed between them, the runtime ABI, and the mid-level IR.

---

## Overview

```
source.lin
  └─ lin-lex       Lexer           source → Token[]
  └─ lin-parse     Parser          Token[] → Module (surface AST)
  └─ lin-check     Type checker    Module → TypedModule
  └─ lin-ir        Lowering        TypedModule → LinModule (flat 3-address IR)
       └─ rc_elide RC elision      LinModule → LinModule (paired Retain/Release removed)
  └─ lin-codegen   LLVM backend    TypedModule → LLVM IR (via inkwell)
       └─ O2 passes                LLVM IR → optimised LLVM IR
       └─ emit .o                  LLVM IR → object file
  └─ cc link       Linker          .o + liblin_runtime.a → native binary
```

The orchestration lives in `crates/lin-compile/src/lib.rs`. `lin-codegen` currently consumes `TypedModule` directly; `LinIR` (`lin-ir`) is available as a pre-pass but not yet wired into the codegen path — it is used by the RC elision pass and future analyses.

---

## Stage 1 — Lexer (`lin-lex`)

The lexer produces a flat `Vec<Token>` from source text. Each token carries a `Span` (byte-offset range in the source).

Key behaviours:
- Synthetic `Indent` / `Dedent` tokens track block boundaries at two-space level changes.
- Indentation synthesis is suppressed inside balanced `{ }`, `( )`, `[ ]` — allowing multi-line object literals and function arguments without spurious block tokens (ADR-004, ADR-017).
- `InterpString(Vec<InterpPart>)` is a single compound token whose `Expr` parts each carry their own sub-token stream; the parser recurses into them (ADR-005).

---

## Stage 2 — Parser (`lin-parse`)

The parser is a hand-written recursive-descent parser that produces a `Module` (surface AST). Every AST node carries a `Span`.

**Error recovery**: After a parse error on a statement, the parser calls `synchronize()`, which discards tokens until it reaches a statement boundary (`Newline`, `Dedent`, or a keyword that starts a new statement). This allows multiple errors to be reported per compile run rather than halting after the first.

**"Did you mean" diagnostics**: The parser detects and emits context-specific `help` annotations for common mistakes:
- Unquoted object keys: `{ name: 1 }` → suggests `"name"`
- `=` in expression context → suggests `==`
- Missing `else` on an `if` expression

---

## Stage 3 — Type Checker (`lin-check`)

The type checker validates the surface AST and produces a `TypedModule` containing a typed version of every expression.

### Input / output

```rust
Checker::new().check_module(&module) -> Result<TypedModule, Vec<Diagnostic>>
```

`TypedModule` is serialisable (`serde` + `bincode`) for the module cache.

### Type representation (`types.rs`)

```
Type::Null | Bool
     | Int8 | Int16 | Int32 | Int64
     | UInt8 | UInt16 | UInt32 | UInt64
     | Float32 | Float64
     | Str
     | Array(Box<Type>)
     | FixedArray(Vec<Type>)
     | Object(IndexMap<String, Type>)
     | Union(Vec<Type>)
     | Function { params: Vec<Type>, ret: Box<Type> }
     | Iterator(Box<Type>)
     | TypeVar(u32)          -- unification variable; must be solved before codegen
     | Never                 -- bottom type (unreachable branches)
```

### Typed IR (`typed_ir.rs`)

Every expression is lowered from `Expr` (surface) to `TypedExpr` (typed):

```
TypedExpr::IntLit(i64, Type, Span)
         | FloatLit(f64, Type, Span)
         | StringLit(String, Span)
         | BoolLit(bool, Span)
         | NullLit(Span)
         | LocalGet { slot: usize, ty: Type, span }
         | LocalSet { slot: usize, value: Box<TypedExpr>, ty, span }
         | BinaryOp { left, op: BinOp, right, result_type, span }
         | Coerce { expr, from: Type, to: Type, span }    -- implicit numeric widening
         | Call { func, args, result_type, is_tail: bool, span }
         | If { cond, then_br, else_br, result_type, span }
         | Match { scrutinee, arms, result_type, span }
         | Block { stmts, expr, ty, span }
         | Function { params, body, ret_type, captures, span }
         | MakeObject { fields, spreads, ty, span }
         | MakeArray { elements, ty, span }
         | Index { object, key, result_type, span }
         | FieldGet { object, field, result_type, span }
         | StringInterp { parts, span }
         | Is { expr, pattern, span }
         | Has { expr, pattern, span }
```

Variables are **slot-indexed** rather than name-indexed. `LocalGet { slot }` refers to the binding at slot `slot`; closures record the `outer_slot` of each captured variable. This eliminates name-lookup overhead in codegen.

### Key checker passes

| Pass | Where | What |
|------|-------|-------|
| Type resolution | `resolve.rs` | Converts surface `TypeExpr` → internal `Type`; handles generic substitution |
| Bidirectional inference | `checker.rs` | `check(expr, expected)` pushes types down; `infer(expr) → Type` synthesises bottom-up |
| Numeric widening | `widen.rs` | Binary ops emit a `Coerce` node when one operand needs widening |
| Structural compatibility | `compat.rs` | `is_compatible(value_ty, target_ty)` — used for assignments, call-site checking |
| Flow-sensitive narrowing | `checker.rs` | After `is`/`has` tests, refines union types in true branches |
| Exhaustiveness checking | `exhaustiveness.rs` | Maranget matrix-decomposition algorithm; produces a counterexample witness if non-exhaustive |
| TypeVar zonking | `zonk.rs` | Post-inference walk that replaces all solved `TypeVar(id)` with their concrete types; unsolved vars are errors |
| Closure analysis | `checker.rs` | Identifies free variables; mutable `var` captures become heap cells (`is_mutable: true`) |

### "Did you mean" diagnostics

For undefined variables, the checker computes the Wagner-Fischer edit distance between the missing name and all names in scope, and emits a `help` annotation if any is within distance 2. The same mechanism fires for missing object fields on known-shape types.

### Module cache

`lin-compile` caches `TypedModule` by source hash:

```
.lin-cache/
  <sha256(source)>.typed    -- bincode-serialised TypedModule
  <sha256(source)>.sig      -- bincode-serialised ModuleSignature
```

A `ModuleSignature` is just the exported `name → Type` map. Dependents only need the signature to verify their own usage; if the signature is unchanged after a source edit, dependents are not re-checked even if the implementation changed (analogous to Haskell `.hi` files).

---

## Stage 4 — Flat IR (`lin-ir`)

`LinIR` is a flat 3-address IR sitting between `TypedExpr` (tree-shaped) and LLVM. It enables analyses that need explicit control flow (liveness, RC elision) without going all the way to LLVM IR.

### Design principles

- **No nested expressions**: every sub-expression result is a named `Temp(u32)`.
- **No phi nodes**: merge-points use explicit `Copy` instructions to pre-allocated temps; SSA reconstruction is delegated to LLVM.
- **Explicit RC**: `Retain { val, ty }` and `Release { val, ty }` instructions are emitted by the lowering pass and removed by the RC elision pass where provably redundant.
- **Flat basic blocks**: each `BasicBlock` is a list of `Instruction`s ending with exactly one `Terminator`.

### Identity types

```rust
Temp(u32)     -- SSA value within a function
BlockId(u32)  -- basic block within a function
FuncId(u32)   -- function within a module
```

### Instructions

```
Const { dst, val: Const }
Copy { dst, src }
Unary { dst, op, operand, ty }
Binary { dst, op: BinOp, lhs, rhs, ty }
Coerce { dst, src, from_ty, to_ty }
Call { dst, callee: CallTarget, args, ret_ty }
CallIntrinsic { dst, intrinsic: Intrinsic, args, ret_ty }
MakeClosure { dst, func: FuncId, captures, ret_ty }
MakeObject { dst, fields, spreads, ty }
MakeArray { dst, elements, elem_ty }
Index { dst, object, key, result_ty }
FieldGet { dst, object, field, result_ty }
Retain { val, ty }
Release { val, ty }
IsType { dst, val, ty }
HasPattern { dst, val, pattern: HasDesc }
Box { dst, val, ty }
Unbox { dst, val, result_ty }
Bind { dst, src, ty }
Panic { msg }
```

### Terminators

```
Return(Option<Temp>)
Jump(BlockId)
CondJump { cond, then_block, else_block }
Switch { val, cases: Vec<(u8, BlockId)>, default }   -- O(1) tag dispatch
TailCall { args }                                     -- TCO loop-back
Unreachable
```

### Lowering pass (`lower.rs`)

`lower_module(typed: &TypedModule) -> LinModule` performs a single tree-walk:

- Statements → sequences of instructions in the current block.
- `If` → `CondJump` to then/else blocks, results copied into a pre-allocated merge temp, then `Jump` to a merge block.
- `Match` → a sequence of pattern-test blocks, each with a `CondJump` to body or next arm; exhaustion falls through to a `Panic`.
- Nested `Function` nodes → lifted into new `LinFunction`s pushed to `ctx.pending_functions`; the outer function emits `MakeClosure`.
- `StringInterp` → a chain of `CallIntrinsic(ToString)` + `CallIntrinsic(StringConcat)` instructions with `Release` for intermediate temps.

### Liveness analysis (`liveness.rs`)

Standard backwards dataflow over basic blocks:

```
live_in[b]  = use[b] ∪ (live_out[b] − def[b])
live_out[b] = ∪ { live_in[s] | s ∈ successors(b) }
```

Iterated to fixpoint. Also computes per-instruction `live_before` sets (live set immediately before each instruction) used by the RC elision pass.

### RC elision pass (`rc_elide.rs`)

Implements a conservative approximation of Perceus (Reinking et al., PLDI 2021):

For each `Retain(t)` in a block, search forward for its matched `Release(t)`. If the path between them contains no call, heap allocation (`MakeObject`, `MakeArray`, `MakeClosure`), or another `Release(t)`, both instructions are removed — the value was never shared across a potential reuse point, so the retain/release pair was a no-op.

Only types that participate in RC (`Str`, `Array`, `Object`, `Function`) are candidates.

---

## Stage 5 — LLVM Codegen (`lin-codegen`)

The codegen crate uses [`inkwell`](https://github.com/TheDan64/inkwell) (safe Rust bindings to the LLVM C API) to compile `TypedModule` directly to LLVM IR.

### Type mapping

| Lin type | LLVM type |
|----------|-----------|
| `Bool` | `i1` |
| `Int8` / `UInt8` | `i8` |
| `Int16` / `UInt16` | `i16` |
| `Int32` / `UInt32` | `i32` |
| `Int64` / `UInt64` | `i64` |
| `Float32` | `float` |
| `Float64` | `double` |
| `Str` | `ptr` (to heap-allocated `LinString`) |
| `Array(T)` | `ptr` (to heap-allocated `LinArray`) |
| `Object(...)` | `ptr` (to heap-allocated `LinObject`) |
| `Union(...)` | `ptr` (to heap-allocated `TaggedVal`) |
| `Function { ... }` | `ptr` (to `{ fn_ptr, env_ptr }` closure pair) |
| `Null` | `i8` (constant `0`) |

### Slot storage

Each Lin binding slot maps to a `SlotStorage`:

```rust
enum SlotStorage {
    Value(BasicValueEnum),   // immutable val — bare SSA value
    Alloca(PointerValue),    // mutable var — stack alloca (heap cell for var captures)
    Closure(PointerValue),   // closure value — ptr to { fn_ptr, env_ptr }
}
```

### Functions and closures

Pure (non-capturing) functions are compiled as top-level LLVM functions. Closures are compiled with an implicit first parameter `env_ptr: ptr` that points to a heap-allocated struct containing captured values. At call sites, the callee is called as `callee(env_ptr, args...)`.

For mutual recursion, top-level named functions are forward-declared before their bodies are compiled (matching ADR-015).

### Tail-call optimisation

Direct self-recursive tail calls are transformed into a loop: the function body is wrapped in an unconditional `br` back to a `loop_body` block; self-tail-calls store updated arguments into alloca slots and branch back. No trampoline is needed.

### Union types and tagged dispatch

Values of union type (`Union(...)`) are boxed into a heap-allocated `TaggedVal`:

```c
struct TaggedVal {
    uint8_t tag;    // 0=null, 1=bool, 2=int32, 3=int64, 4=float64, 5=str, 6=object, 7=array, 8=function
    uint8_t pad[7];
    uint64_t payload;  // integer/float value, or pointer
};
```

`match` over a union type emits an LLVM `switch i8 %tag` (O(1) jump table) rather than a chain of `icmp`/`br` pairs.

### Unboxed scalar arrays

When the element type of an array is a known concrete scalar (`Int32`, `UInt32`, `Int64`, `UInt64`, `Float32`, `Float64`), the codegen emits flat (unboxed) array operations:

| Scalar type | Alloc | Push | Get |
|-------------|-------|------|-----|
| `i32` / `u32` | `lin_flat_array_alloc_i32` | `lin_flat_array_push_i32` | `lin_flat_array_get_i32` |
| `i64` / `u64` | `lin_flat_array_alloc_i64` | `lin_flat_array_push_i64` | `lin_flat_array_get_i64` |
| `f32` | `lin_flat_array_alloc_f32` | `lin_flat_array_push_f32` | `lin_flat_array_get_f32` |
| `f64` | `lin_flat_array_alloc_f64` | `lin_flat_array_push_f64` | `lin_flat_array_get_f64` |

Flat arrays store raw scalars (4 or 8 bytes per element) instead of `LinArrayElem` tagged unions (16 bytes each), giving a 5–10× improvement in numeric array access.

---

## Stage 6 — Runtime Library (`lin-runtime`)

A small static library (`liblin_runtime.a`) linked into every compiled binary. Written in Rust and compiled to a `staticlib` crate target.

### Memory model

Heap values use **reference counting**. The refcount is stored as the first `u32` field of every heap struct. `lin_alloc(size)` is a thin wrapper over `malloc`. `lin_rc_retain` / `lin_rc_release` adjust the count.

Strings, arrays, and objects each have type-specific `*_release` functions that decrement the refcount, free the struct if zero, and recursively release nested values.

### Data structures

**`LinString`** (`string.rs`):
```c
struct LinString {
    uint32_t refcount;
    uint32_t len;       // byte length (UTF-8)
    uint8_t  data[];    // inline UTF-8 bytes (no null terminator)
};
```

**`LinArray`** (`array.rs`):
```c
struct LinArray {
    uint32_t refcount;
    uint32_t _pad;
    uint64_t len;
    uint64_t cap;
    LinArrayElem *data;  // pointer to element storage
};
// Tagged element (default — used when element type is not a flat scalar):
struct LinArrayElem {
    uint8_t  tag;
    uint8_t  pad[7];
    uint64_t payload;   // value or pointer
};
// Flat scalar arrays reuse LinArray with data pointing to raw T[] instead.
```

**`LinObject`** (`object.rs`):
```c
struct LinObject {
    uint32_t refcount;
    uint32_t len;
    uint32_t cap;
    LinObjectEntry *entries;  // sorted by key pointer for binary search
};
```

**`TaggedVal`** (`tagged.rs`):
```c
struct TaggedVal {
    uint8_t  tag;
    uint8_t  pad[7];
    uint64_t payload;
};
```

### C-ABI surface (selected)

All functions use the C calling convention (`extern "C"`).

**I/O**
```
lin_print(s: *LinString) -> void
lin_panic(msg: *LinString, file_id: i32, offset: i32) -> void  (noreturn)
```

**Strings**
```
lin_string_from_bytes(data: *u8, len: u32) -> *LinString
lin_string_concat(a, b: *LinString) -> *LinString
lin_string_length(s: *LinString) -> i32
lin_string_eq(a, b: *LinString) -> bool
lin_string_release(s: *mut LinString) -> void
lin_string_slice(s, start, end) -> *LinString
lin_string_char_at(s, i) -> *LinString
lin_string_trim / to_upper / to_lower / index_of / contains
lin_string_starts_with / ends_with / replace / repeat / split / join
lin_int_to_string(n: i64) -> *LinString
lin_float_to_string(f: f64) -> *LinString
lin_bool_to_string(b: bool) -> *LinString
lin_null_to_string() -> *LinString
lin_tagged_to_string(tv: *TaggedVal) -> *LinString
```

**Arrays**
```
lin_array_alloc(cap: u64) -> *LinArray
lin_array_push(arr, elem_ptr, tag) -> void
lin_array_get(arr, idx: i64) -> *LinArrayElem
lin_array_length(arr) -> i64
lin_array_release(arr: *mut LinArray) -> void
-- flat scalar variants for i32, i64, f32, f64:
lin_flat_array_alloc_{i32,i64,f32,f64}(cap: u64) -> *LinArray
lin_flat_array_push_{i32,i64,f32,f64}(arr, val) -> void
lin_flat_array_get_{i32,i64,f32,f64}(arr, idx: i64) -> T
lin_flat_array_free_{i32,i64,f32,f64}(arr: *mut LinArray) -> void
```

**Objects**
```
lin_object_alloc(initial_cap: u32) -> *LinObject
lin_object_set(obj, key: *LinString, val: *TaggedVal) -> void
lin_object_get(obj, key: *LinString) -> *TaggedVal (null if missing)
lin_object_has(obj, key: *LinString) -> u8
lin_object_eq(a, b: *LinObject) -> u8
lin_object_release(obj: *mut LinObject) -> void
```

**Tagged unions**
```
lin_box_null() -> *u8
lin_box_bool(v: u8) -> *u8
lin_box_int32(v: i32) -> *u8
lin_box_int64(v: i64) -> *u8
lin_box_float64(v: f64) -> *u8
lin_box_str/object/array/function(p: *u8) -> *u8
lin_get_tag(p: *u8) -> u8
lin_unbox_int32/int64/float64/bool/ptr(p: *u8) -> T
lin_tagged_release(p: *mut u8) -> void
```

**Numbers**
```
lin_parse_int32(s: *LinString) -> i32
lin_parse_float64(s: *LinString) -> f64
lin_to_int32(v: f64) -> i32
lin_to_float64(v: i32) -> f64
lin_is_int32(s: *LinString) -> bool
```

**Memory**
```
lin_alloc(size: usize) -> *u8
lin_rc_retain(ptr: *u32) -> void
lin_rc_release(ptr: *u32, size: usize, align: usize) -> void
```

---

## Stage 7 — Linking

After codegen emits a `.o` object file, `lin-compile` invokes `cc` to link it with `liblin_runtime.a` into a standalone native binary. The runtime library is found by searching the standard cargo target directories (`target/debug/`, `target/release/`, etc.).

The `.o` file is deleted after a successful link. `LIN_EMIT_IR=1` writes the `.ll` LLVM IR file alongside the binary before linking.

---

## Compilation pipeline entry points

| Command | Entry |
|---------|-------|
| `lin build file.lin [-o out]` | `lin_compile::compile(&CompileOptions)` |
| `lin check file.lin` | `lin_check::Checker::check_module` |
| `lin run file.lin` | `lin_eval::Interpreter::run_file` |

`CompileOptions`:
```rust
pub struct CompileOptions {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub emit_ir: bool,    // set by LIN_EMIT_IR=1
    pub optimize: bool,   // cleared by LIN_NO_OPT=1
}
```

---

## Extending the compiler

### Adding a new type

1. Add a variant to `Type` in `lin-check/src/types.rs`.
2. Add the `Display` arm and serde derives (already derived).
3. Handle it in `resolve.rs` (surface `TypeExpr` → `Type`), `compat.rs` (compatibility), `widen.rs` (numeric widening if applicable), and `checker.rs` (infer/check).
4. Add the LLVM type mapping in `codegen.rs`:`llvm_type()`.
5. Add runtime support in `lin-runtime` if heap allocation is needed.

### Adding a new intrinsic

1. Register the name and arity in `lin-check/src/checker.rs`:`register_intrinsics()`.
2. Add a `compile_intrinsic_call` arm in `lin-codegen/src/codegen.rs`.
3. If it needs a new runtime function: add it to the appropriate `lin-runtime/src/*.rs` module and declare it in `Codegen::new()`.
4. Add the `Intrinsic` variant to `lin-ir/src/ir.rs` if the IR lowering pass should handle it explicitly.

### Adding a new IR instruction

1. Add the variant to `Instruction` in `lin-ir/src/ir.rs`.
2. Add arms in `liveness.rs`:`instr_use_def()` (return the correct `(uses, defs)`).
3. Handle it in `rc_elide.rs`:`path_has_no_interference()` if it could alias a refcounted value.
4. Add lowering in `lower.rs` and eventually a codegen arm when `lin-codegen` consumes `LinIR`.
