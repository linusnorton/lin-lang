# Lin

A small, expression-based programming language built around JSON data, structural typing, pattern matching, and functional-style pipelines.

```
val greet = (name: String): String => "Hello, ${name}!"
print(greet("world"))
```

## Design

- Everything is an expression
- Runtime values are strict JSON (null, bool, number, string, array, object) plus functions and iterators
- Indentation defines blocks — no braces or semicolons
- Structural typing with union types and pattern matching
- Errors are ordinary values
- Dot syntax for chaining: `x.f(y)` calls `f(x, y)`

## Getting Started

**Prerequisites:** Rust toolchain, LLVM 18+ (tested with LLVM 22), a C linker (`cc`).

```bash
git clone <repo>
cd lin-lang
cargo build --workspace
```

## Running Programs

### Compile to a native binary

```bash
cargo run -p lin -- build examples/hello.lin -o hello
./hello
```

Specify a custom output path with `-o`. The default output name is derived from the source file.

```bash
lin build myprogram.lin -o myprogram
./myprogram
```

### Type check only

```bash
lin check examples/hello.lin
```

Reports type errors without producing a binary.

## Language Tour

### Values and bindings

```lin
val x = 42
val name = "Alice"
val active = true
val nothing = null
```

`val` bindings are immutable. Use `var` for mutable bindings:

```lin
var count = 0
count = count + 1
```

### Functions

```lin
val add = (a: Int32, b: Int32): Int32 => a + b

print(toString(add(3, 4)))   // 7
```

Multi-statement bodies use indentation:

```lin
val gradeFor = (avg: Int32): String =>
  match avg
    has Int32 when avg >= 90 => "A"
    has Int32 when avg >= 80 => "B"
    has Int32 when avg >= 70 => "C"
    else => "F"
```

### Dot chaining

`x.f(y)` is sugar for `f(x, y)`:

```lin
val result = "  hello  ".trim().toUpper()
print(result)   // HELLO
```

### Pattern matching

```lin
val describe = (input: String | Int32 | Null): String =>
  match input
    is Null    => "nothing"
    is Int32   => "an integer"
    is String  => "a string"
```

Destructure objects with `has`:

```lin
val describePerson = (p: Json): String =>
  match p
    has { name, age } when age > 30 => "Old: ${name}"
    has { name }                     => "Young: ${name}"
    else                             => "unknown"
```

### Arrays and pipelines

```lin
val numbers = [1, 2, 3, 4, 5]

val evens = numbers.filter(n => n % 2 == 0)
val doubled = evens.map(n => n * 2)
val total = doubled.reduce(0, (acc, n) => acc + n)

print(toString(total))   // 12
```

### String interpolation

```lin
val name = "Lin"
val version = 1
print("${name} v${toString(version)}")
```

### Imports

```lin
import { trim, toUpper } from "std/string"
import { parseInt32 }    from "std/number"
import { print }         from "std/io"
import { square }        from "lib/math"   // relative path
```

### Value-based error handling

```lin
val divide = (a: Float64, b: Float64): Json =>
  if b == 0.0
    then { "type": "failure", "error": "division by zero" }
    else { "type": "success", "value": a / b }

val result = divide(10.0, 2.0)
val message = match result
  has { "type": "success", value } => "Result: ${toString(value)}"
  has { "type": "failure", error } => "Error: ${error}"

print(message)
```

## Standard Library

| Module | Exports |
|---|---|
| `std/io` | `print`, `readLine`, `readAll`, `lines` |
| `std/string` | `trim`, `toUpper`, `toLower`, `split`, `join`, `contains`, `replace`, `startsWith`, `endsWith`, `indexOf`, `charAt`, `repeat` |
| `std/number` | `parseInt32`, `parseFloat64`, `isInt32`, `toInt32`, `toFloat64` |
| `std/array` | `map`, `filter`, `reduce`, `for`, `range`, `length`, `push`, `concat` |
| `std/iter` | `iter`, `range`, iterator combinators |
| `std/result` | `Result<T, E>` type and helpers |
| `std/fs` | `readFile`, `writeFile`, `appendFile`, `readLines`, `readJson`, `writeJson`, `exists` |
| `std/http` | `fetch`, `fetchWith`, `fetchJson`, `postJson` |
| `std/server` | `serve`, `json`, `text`, `redirect`, `notFound`, `badRequest`, `parseBody`, `pathMatch` |

### Concurrency

```lin
// Spawn a background task
val p = async(() =>
  val result = fetchJson("https://api.example.com/data")
  result["value"]
)

// Block until done
val value = await(p)
print(toString(value))

// Fork-join: run three tasks in parallel, collect results in order
val results = parallel(
  () => computeA(),
  () => computeB(),
  () => computeC()
)

// Thread pool
val pool = threadPool(4)
val promises = pool.async([() => task1(), () => task2()])
val done = await(promises)
```

Workers handle messages on a dedicated thread:

```lin
val counter = worker(
  (msg) =>
    var n = 0
    n = n + msg
    n
  ,
  () => null
)
counter.message(1)
val total = counter.request(5)   // blocks until reply
counter.close()
```

### HTTP

```lin
import { fetchJson, postJson } from "std/http"

val data = fetchJson("https://api.example.com/items")
data.for(item => print(item["name"]))

val response = postJson("https://api.example.com/create", { "name": "Lin" })
print(toString(response["status"]))
```

HTTP server:

```lin
import { serve, json, text, pathMatch } from "std/server"

serve(8080, req =>
  match pathMatch("/users/:id", req["path"])
    is Null => text(404, "not found")
    has { id } => json(200, { "userId": id })
)
```

### Foreign Functions (C / Rust interop)

Call functions from compiled C or Rust static libraries. Requires `lin build`.

```lin
import foreign "libmathlib.a"
  val sqrt: (Float64) => Float64
  val add: (Int32, Int32) => Int32

print(toString(sqrt(2.0)))   // 1.4142...
print(toString(add(3, 4)))   // 7
```

The C header `crates/lin-runtime/lin.h` defines `LinString` and `LinArray` for passing non-primitive types across the boundary. See `examples/ffi_c.lin` for a complete example.

## Project Layout

```
crates/
  lin-common/   shared Span, Diagnostic, edit-distance helpers
  lin-lex/      lexer
  lin-parse/    parser and surface AST (with error recovery)
  lin-check/    type checker — produces TypedModule (typed IR)
  lin-ir/       flat 3-address IR, liveness analysis, RC elision pass
  lin-codegen/  LLVM backend (via inkwell)
  lin-runtime/  runtime library linked into compiled binaries (+ lin.h FFI header)
  lin-compile/  compilation pipeline (lex → parse → check → codegen → link)
  lin-eval/     tree-walking interpreter (lin run)
  lin/          CLI binary
  lin-lsp/      language server (in progress)
stdlib/         standard library source files (.lin)
examples/       example programs
docs/           specification and design decisions
.github/        CI workflow (cargo test + examples)
```

## Development

```bash
cargo test --workspace                        # run all tests
cargo test -p lin-eval test_hello_world       # run a single test
cargo run -p lin -- run examples/showcase.lin # interpret an example
cargo run -p lin -- build examples/showcase.lin -o /tmp/showcase && /tmp/showcase
```

Set `LIN_EMIT_IR=1` to write the LLVM IR alongside the compiled binary (useful for debugging):

```bash
LIN_EMIT_IR=1 lin build myprogram.lin -o myprogram
# produces myprogram and myprogram.ll
```
