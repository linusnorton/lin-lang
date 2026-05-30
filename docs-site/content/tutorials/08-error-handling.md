# Error Handling

Lin has no exceptions. There is no `throw`, no `try/catch`, no implicit error propagation. Errors are ordinary values — functions that can fail return a union type that includes the error case.

## Why no exceptions?

Exceptions make control flow invisible. A function that throws can disrupt the caller without the caller's type signature saying anything about it. Lin makes failures explicit at the type level: if a function can fail, its return type says so.

## The tagged union pattern

The idiomatic pattern for fallible operations is to return a tagged object:

```lin
type Result<T, E> =
  | { "type": "success", "value": T }
  | { "type": "failure", "error": E }
```

A function that might fail:

```lin
import { isInt32, parseInt32 } from "std/number"

val parseAge = (s: String): Json =>
  if isInt32(s) then
    val n = parseInt32(s)
    if n >= 0 && n <= 150 then { "type": "success", "value": n }
    else { "type": "failure", "error": "age out of range: ${s}" }
  else
    { "type": "failure", "error": "not a number: ${s}" }
```

## Handling the result

Use `match`/`has` to inspect the result:

```lin
import { print } from "std/io"

val result = parseAge("25")
match result
  has { "type": "success", value } =>
    print("age is ${value}")
  has { "type": "failure", error } =>
    print("error: ${error}")
```

## Composing fallible operations

Chain results by matching each step:

```lin
import { readFile } from "std/fs"
import { readJson } from "std/fs"

val loadConfig = (path: String): Json =>
  val fileResult = readFile(path)
  match fileResult
    has { "type": "failure", error } =>
      { "type": "failure", "error": "cannot read config: ${error}" }
    else =>
      val parseResult = readJson(path)
      match parseResult
        has { "type": "failure", error } =>
          { "type": "failure", "error": "cannot parse config: ${error}" }
        else =>
          { "type": "success", "value": parseResult }
```

## Standard library errors

The standard library (`std/fs`, `std/http`, etc.) returns `Json | Error` or `T | Error`. Match on the result to handle failures:

```lin
import { readFile } from "std/fs"
import { print } from "std/io"

val src = readFile("data.txt")
match src
  has { "type": "failure", error } => print("could not read: ${error}")
  else => print("file contents: ${src}")
```

## Runtime errors

A small number of operations halt the program without recovery:

- Array index out of bounds
- Integer division by zero
- Non-exhaustive `match` (no arm matched, no `else`)

These cannot be caught. They indicate programming errors, not expected conditions. For expected failure modes, use a union return type.

## Async fault isolation

The one place where runtime errors become recoverable values is inside `async` thunks:

```lin
import { async, await } from "std/async"

val p = async(() => riskyOperation())
val result = await(p)
match result
  is Error => print("async task failed")
  else     => print("success: ${result}")
```

A runtime error inside the thunk is caught at the thread boundary and surfaces as an `Error` value at the `await` call site.
