# Lin — a language that gets out of your way

Lin is a compiled, functional-leaning language that feels like a modern scripting language. It combines a clean minimal syntax with a maximalist standard library, structural typing, union types, and value-based error handling — so you can build performant systems utilities and full application services without switching languages or compromising on correctness.

---

## A taste of Lin

```lin
import { print } from "std/io"
import { map, filter, reduce } from "std/array"
import { trim, split } from "std/string"

// Parse CSV lines into structured records
val parseRecord = (line: String): Json =>
  val parts = split(line, ",")
  if length(parts) < 3 then { "type": "failure", "error": "bad line: ${line}" }
  else {
    "type": "success",
    "value": {
      "name": trim(parts[0]),
      "age": trim(parts[1]),
      "city": trim(parts[2])
    }
  }

val data = [
  "Alice, 30, London",
  "Bob, 25, New York",
  "bad-line",
  "Charlie, 35, Berlin"
]

// Pipeline: parse, filter successes, find adults, collect names
val names = data
  .map(line => parseRecord(line))
  .filter(r => r["type"] == "success")
  .map(r => r["value"])
  .filter(p => p["age"] >= 30)
  .map(p => p["name"])

names.for(name => print(name))
// Alice
// Charlie
```

---

## Why Lin?

- **JSON-native data model.** Objects, arrays, strings, numbers, and null are first-class. No impedance mismatch between your data and your code.
- **Structural typing.** Types describe the shape of data. A `{ "name": String, "age": Int32 }` goes anywhere that shape is expected — no casting, no adapters.
- **Union types and pattern matching.** Model every possible outcome explicitly. The compiler checks that you handle them all.
- **No exceptions.** Errors are values. Functions that can fail return a union. You match on the result and handle it explicitly.
- **Async without colouring.** `async` and `await` are ordinary function calls. No `async def`, no coloured functions, no viral infection of your call stack.
- **Compiled to native.** `lin build` produces a standalone native binary via LLVM. No runtime, no VM, no interpreter to distribute.

---

## Get started

[Read the Getting Started guide](/getting-started.html) to install Lin, write your first program, and explore the language in 15 minutes.

Or jump straight into the tutorials:

- [Hello World & I/O](/tutorials/01-hello-world.html) — your first Lin program
- [Values & Types](/tutorials/02-values-and-types.html) — understanding the type system
- [Functions](/tutorials/03-functions.html) — first-class functions and closures
- [Pattern Matching](/tutorials/05-pattern-matching.html) — the primary way to inspect values
