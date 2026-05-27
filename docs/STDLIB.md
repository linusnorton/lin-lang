# Lin Standard Library Specification

This document specifies the standard library for the Lin language. All modules are importable via the `std/` prefix.

## Index

### Modules

| Module | Description |
| --- | --- |
| [`std/string`](#stdstring) | String manipulation functions |
| [`std/array`](#stdarray) | Array transformation and query functions |
| [`std/iter`](#stditer) | Iterator constructors (auto-imported as globals) |
| [`std/number`](#stdnumber) | Numeric parsing and conversion functions |
| [`std/result`](#stdresult) | Result type for error handling |
| [`std/io`](#stdio) | stdin/stdout and terminal input |
| [`std/fs`](#stdfs) | Filesystem read and write |
| [`std/http`](#stdhttp) | HTTP client |
| [`std/server`](#stdserver) | HTTP server |
| [`std/template`](#stdtemplate) | String template rendering |
| [`std/test`](#stdtest) | Test framework |

### Functions by module

**std/string**

| Function | Signature | Summary |
| --- | --- | --- |
| [`trim`](#trim) | `(String) -> String` | Remove leading and trailing whitespace |
| [`toUpper`](#toUpper) | `(String) -> String` | Convert to uppercase |
| [`toLower`](#toLower) | `(String) -> String` | Convert to lowercase |
| [`substring`](#substring) | `(String, Int32, Int32) -> String` | Extract a slice by codepoint indices |
| [`charAt`](#charAt) | `(String, Int32) -> String` | Single-codepoint string at index |
| [`indexOf`](#indexOf-string) | `(String, String) -> Int32` | First occurrence of needle, or -1 |
| [`length`](#length-string) | `(String) -> Int32` | Codepoint count |
| [`contains`](#contains) | `(String, String) -> Boolean` | Test whether needle is a substring |
| [`startsWith`](#startsWith) | `(String, String) -> Boolean` | Test whether string begins with prefix |
| [`endsWith`](#endsWith) | `(String, String) -> Boolean` | Test whether string ends with suffix |
| [`split`](#split) | `(String, String) -> String[]` | Split by delimiter |
| [`join`](#join) | `(String[], String) -> String` | Join array of strings with separator |
| [`replace`](#replace) | `(String, String, String) -> String` | Replace first occurrence |
| [`repeat`](#repeat) | `(String, Int32) -> String` | Repeat a string n times |

**std/array**

| Function | Signature | Summary |
| --- | --- | --- |
| [`map`](#map) | `(Json[], (Json) -> Json) -> Json[]` | Transform each element |
| [`filter`](#filter) | `(Json[], (Json) -> Boolean) -> Json[]` | Keep elements matching predicate |
| [`reduce`](#reduce) | `(Json[], Json, (Json, Json) -> Json) -> Json` | Fold left with an accumulator |
| [`find`](#find) | `(Json[], (Json) -> Boolean) -> Json` | First matching element, or null |
| [`some`](#some) | `(Json[], (Json) -> Boolean) -> Boolean` | True if any element matches |
| [`every`](#every) | `(Json[], (Json) -> Boolean) -> Boolean` | True if all elements match |
| [`flatMap`](#flatMap) | `(Json[], (Json) -> Json[]) -> Json[]` | Map then flatten one level |
| [`indexOf`](#indexOf-array) | `(Json[], Json) -> Int32` | First index of value, or -1 |
| [`reverse`](#reverse) | `(Json[]) -> Json[]` | Return a reversed copy |

**std/iter**

| Function | Signature | Summary |
| --- | --- | --- |
| [`range`](#range) | `(Int32, Int32) -> Iterator` | Integer range [start, end) |
| [`iterOf`](#iterOf) | `(Json[]) -> Iterator` | Iterator over an array |

**std/number**

| Function | Signature | Summary |
| --- | --- | --- |
| [`parseInt32`](#parseInt32) | `(String) -> Int32` | Parse decimal string to Int32 |
| [`parseFloat64`](#parseFloat64) | `(String) -> Float64` | Parse decimal string to Float64 |
| [`toInt32`](#toInt32) | `(Float64) -> Int32` | Truncate float to Int32 |
| [`toFloat64`](#toFloat64) | `(Int32) -> Float64` | Widen Int32 to Float64 |
| [`isInt32`](#isInt32) | `(String) -> Boolean` | Test whether a string parses as Int32 |

**std/result**

| Name | Kind | Summary |
| --- | --- | --- |
| [`Result`](#Result) | type | Union of success and failure shapes |

**std/io**

| Function | Signature | Summary |
| --- | --- | --- |
| [`print`](#print) | `(Json) -> Null` | Write a value to stdout |
| [`readLine`](#readLine) | `() -> String \| Null` | Read one line from stdin, or Null on EOF |
| [`lines`](#lines) | `() -> Iterator` | Iterator over stdin lines |
| [`readAll`](#readAll) | `() -> String` | Read all of stdin as one string |

**std/fs**

| Function | Signature | Summary |
| --- | --- | --- |
| [`readFile`](#readFile) | `(String) -> String \| Error` | Read entire file as a string |
| [`writeFile`](#writeFile) | `(String, String) -> Null \| Error` | Write string to file, replacing contents |
| [`appendFile`](#appendFile) | `(String, String) -> Null \| Error` | Append string to end of file |
| [`readLines`](#readLines) | `(String) -> Iterator \| Error` | Iterator over lines of a file |
| [`readJson`](#readJson) | `(String) -> Json \| Error` | Read and parse file as JSON |
| [`writeJson`](#writeJson) | `(String, Json) -> Null \| Error` | Serialise value to JSON and write to file |
| [`exists`](#exists) | `(String) -> Boolean` | Test whether a file or directory exists |

**std/http**

| Function | Signature | Summary |
| --- | --- | --- |
| [`fetch`](#fetch) | `(String) -> HttpResponse \| Error` | GET a URL |
| [`fetchWith`](#fetchWith) | `(String, HttpOptions) -> HttpResponse \| Error` | Request with custom method, headers, body |
| [`fetchJson`](#fetchJson) | `(String) -> Json \| Error` | GET a URL and parse the body as JSON |
| [`postJson`](#postJson) | `(String, Json) -> HttpResponse \| Error` | POST a JSON body to a URL |

**std/server**

| Function | Signature | Summary |
| --- | --- | --- |
| [`serve`](#serve) | `(Int32, (HttpRequest) -> HttpResponse) -> Null` | Start a sequential HTTP server |
| [`json`](#json-helper) | `(Int32, Json) -> HttpResponse` | Build a JSON response |
| [`text`](#text-helper) | `(Int32, String) -> HttpResponse` | Build a plain-text response |
| [`redirect`](#redirect) | `(String) -> HttpResponse` | Build a 302 redirect response |
| [`notFound`](#notFound) | `() -> HttpResponse` | Build a 404 response |
| [`badRequest`](#badRequest) | `(String) -> HttpResponse` | Build a 400 response with a message |
| [`pathMatch`](#pathMatch) | `(String, String) -> { ...String } \| Null` | Match a path pattern, returning captured params |
| [`parseBody`](#parseBody) | `(HttpRequest) -> Json \| Error` | Parse the request body as JSON |

**std/template**

| Function | Signature | Summary |
| --- | --- | --- |
| [`render`](#render) | `(String, {}) -> String \| Error` | Load a `.lint` file and render it with a data record |
| [`renderWith`](#renderWith) | `(String, {}) -> String` | Render a template string with a data record |

**std/test**

| Name | Signature | Summary |
| --- | --- | --- |
| [`suite`](#suite) | `(String, Test[]) -> Suite` | Group tests under a name |
| [`test`](#test) | `(String, () -> Assertion \| Assertion[]) -> Test` | Declare a single test case |
| [`run`](#run) | `(Suite[]) -> Null` | Execute suites, print results, exit non-zero on failure |
| [`expect`](#expect) | `(Json) -> Asserter` | Begin an assertion chain |

---

## std/string

String operations are codepoint-aware. All indices and lengths count Unicode codepoints, not bytes. Byte-level access is not part of this module.

Import:

```txt
import { trim, toUpper, indexOf } from "std/string"
```

---

### trim

```txt
val trim: (s: String) -> String
```

Returns a copy of `s` with all leading and trailing ASCII whitespace characters (`' '`, `'\t'`, `'\n'`, `'\r'`) removed. Interior whitespace is unchanged.

```txt
trim("  hello  ")   // "hello"
trim("\t\n")        // ""
trim("no change")   // "no change"
```

---

### toUpper

```txt
val toUpper: (s: String) -> String
```

Returns a copy of `s` with every codepoint mapped to its Unicode uppercase equivalent.

```txt
toUpper("hello")   // "HELLO"
toUpper("café")    // "CAFÉ"
```

---

### toLower

```txt
val toLower: (s: String) -> String
```

Returns a copy of `s` with every codepoint mapped to its Unicode lowercase equivalent.

```txt
toLower("HELLO")   // "hello"
toLower("CAFÉ")    // "café"
```

---

### substring

```txt
val substring: (s: String, start: Int32, end: Int32) -> String
```

Returns the slice of `s` covering codepoint indices `[start, end)`. Both `start` and `end` are zero-based codepoint offsets. If `end` exceeds the codepoint count of `s`, it is clamped to the end of the string. If `start >= end`, returns `""`.

```txt
substring("hello", 1, 3)   // "el"
substring("hello", 0, 5)   // "hello"
substring("hello", 2, 2)   // ""
```

---

### charAt

```txt
val charAt: (s: String, index: Int32) -> String
```

Returns a single-codepoint string at zero-based codepoint `index`. Equivalent to `substring(s, index, index + 1)`.

```txt
charAt("hello", 0)   // "h"
charAt("hello", 4)   // "o"
```

---

### indexOf (string) {#indexOf-string}

```txt
val indexOf: (s: String, needle: String) -> Int32
```

Returns the zero-based codepoint index of the first occurrence of `needle` within `s`, or `-1` if not found.

```txt
indexOf("hello world", "world")   // 6
indexOf("hello", "xyz")           // -1
indexOf("abcabc", "bc")           // 1
```

---

### length (string) {#length-string}

```txt
val length: (s: String) -> Int32
```

Returns the number of Unicode codepoints in `s`. This is distinct from the byte length, which may be larger for non-ASCII strings.

```txt
length("hello")   // 5
length("")        // 0
length("café")    // 4
```

---

### contains

```txt
val contains: (s: String, needle: String) -> Boolean
```

Returns `true` if `needle` appears anywhere within `s`.

```txt
contains("hello world", "world")   // true
contains("hello", "xyz")           // false
contains("", "")                   // true
```

---

### startsWith

```txt
val startsWith: (s: String, prefix: String) -> Boolean
```

Returns `true` if `s` begins with `prefix`.

```txt
startsWith("hello", "hel")   // true
startsWith("hello", "llo")   // false
startsWith("hello", "")      // true
```

---

### endsWith

```txt
val endsWith: (s: String, suffix: String) -> Boolean
```

Returns `true` if `s` ends with `suffix`.

```txt
endsWith("hello", "llo")   // true
endsWith("hello", "hel")   // false
endsWith("hello", "")      // true
```

---

### split

```txt
val split: (s: String, delimiter: String) -> String[]
```

Splits `s` at each occurrence of `delimiter` and returns the resulting parts as an array. The delimiter is not included in any part. If `s` does not contain `delimiter`, returns a single-element array containing `s`. If `delimiter` is `""`, the behaviour is implementation-defined in v0; callers should avoid splitting on the empty string.

```txt
split("a,b,c", ",")      // ["a", "b", "c"]
split("hello", "x")      // ["hello"]
split("a,,b", ",")       // ["a", "", "b"]
```

---

### join

```txt
val join: (arr: String[], separator: String) -> String
```

Concatenates the elements of `arr` into a single string, with `separator` inserted between each pair of adjacent elements.

```txt
join(["a", "b", "c"], ",")    // "a,b,c"
join(["hello"], "-")           // "hello"
join([], "-")                  // ""
```

---

### replace

```txt
val replace: (s: String, pattern: String, replacement: String) -> String
```

Returns a copy of `s` with the **first** occurrence of `pattern` replaced by `replacement`. If `pattern` does not appear in `s`, returns `s` unchanged.

```txt
replace("hello world", "world", "Lin")   // "hello Lin"
replace("aaa", "a", "b")                 // "baa"
replace("hello", "xyz", "!")             // "hello"
```

---

### repeat

```txt
val repeat: (s: String, count: Int32) -> String
```

Returns a string consisting of `s` repeated `count` times. If `count` is `0`, returns `""`. If `count` is negative, behaviour is a runtime error.

```txt
repeat("ab", 3)   // "ababab"
repeat("x", 0)    // ""
repeat("-", 5)    // "-----"
```

---

## std/array

Array functions operate on `Json[]` values. All functions are non-mutating: they return new arrays and do not modify their inputs.

Import:

```txt
import { map, filter, reduce } from "std/array"
```

---

### map

```txt
val map: (arr: Json[], f: (Json) -> Json) -> Json[]
```

Returns a new array formed by applying `f` to each element of `arr` in order.

```txt
[1, 2, 3].map(x => x * 2)        // [2, 4, 6]
["a", "b"].map(s => toUpper(s))   // ["A", "B"]
```

---

### filter

```txt
val filter: (arr: Json[], f: (Json) -> Boolean) -> Json[]
```

Returns a new array containing only the elements of `arr` for which `f` returns `true`, in their original order.

```txt
[1, 2, 3, 4].filter(x => x > 2)   // [3, 4]
[1, 2, 3].filter(x => x == 2)     // [2]
```

---

### reduce

```txt
val reduce: (arr: Json[], init: Json, f: (Json, Json) -> Json) -> Json
```

Folds `arr` left-to-right, starting from `init`. `f` receives the running accumulator as its first argument and the current element as its second.

```txt
[1, 2, 3, 4].reduce(0, (acc, x) => acc + x)   // 10
[1, 2, 3].reduce(1, (acc, x) => acc * x)       // 6
[].reduce(0, (acc, x) => acc + x)              // 0
```

---

### find

```txt
val find: (arr: Json[], f: (Json) -> Boolean) -> Json
```

Returns the first element of `arr` for which `f` returns `true`, or `null` if no such element exists.

```txt
[1, 2, 3].find(x => x > 1)   // 2
[1, 2, 3].find(x => x > 9)   // null
```

---

### some

```txt
val some: (arr: Json[], f: (Json) -> Boolean) -> Boolean
```

Returns `true` if `f` returns `true` for at least one element of `arr`. Returns `false` for an empty array. Note: `some` does not short-circuit in the current implementation — it visits every element even after a match.

```txt
[1, 2, 3].some(x => x > 2)   // true
[1, 2, 3].some(x => x > 9)   // false
[].some(x => true)            // false
```

---

### every

```txt
val every: (arr: Json[], f: (Json) -> Boolean) -> Boolean
```

Returns `true` if `f` returns `true` for every element of `arr`. Returns `true` for an empty array. Note: `every` does not short-circuit in the current implementation — it visits every element even after a mismatch.

```txt
[1, 2, 3].every(x => x > 0)   // true
[1, 2, 3].every(x => x > 1)   // false
[].every(x => false)           // true
```

---

### flatMap

```txt
val flatMap: (arr: Json[], f: (Json) -> Json[]) -> Json[]
```

Applies `f` to each element of `arr` and concatenates the resulting arrays into a single flat array. Equivalent to mapping then flattening exactly one level.

```txt
[1, 2, 3].flatMap(x => [x, x * 2])   // [1, 2, 2, 4, 3, 6]
[[1, 2], [3]].flatMap(x => x)         // [1, 2, 3]
```

---

### indexOf (array) {#indexOf-array}

```txt
val indexOf: (arr: Json[], target: Json) -> Int32
```

Returns the zero-based index of the first element in `arr` that is deeply equal to `target`, or `-1` if not found. Equality uses the same semantics as the `==` operator (see §24.5 of the specification).

```txt
[10, 20, 30].indexOf(20)       // 1
["a", "b", "c"].indexOf("b")   // 1
[1, 2, 3].indexOf(9)           // -1
```

---

### reverse

```txt
val reverse: (arr: Json[]) -> Json[]
```

Returns a new array with the elements of `arr` in reversed order. Does not modify `arr`.

```txt
[1, 2, 3].reverse()     // [3, 2, 1]
["a"].reverse()          // ["a"]
[].reverse()             // []
```

---

## std/iter

The `range` and `iterOf` functions from `std/iter` are automatically imported into the global scope — programs can use them without an explicit import. They are also re-importable explicitly if desired.

Import (explicit, optional):

```txt
import { range, iterOf } from "std/iter"
```

Iterators are opaque runtime values. They can be consumed once with the built-in `for` function (or the `.for(f)` dot-call form) and cannot be reset or iterated multiple times.

---

### range

```txt
val range: (start: Int32, end: Int32) -> Iterator
```

Returns an iterator that yields the integers `start, start+1, ..., end-1`. The range is half-open: `start` is included, `end` is excluded. If `start >= end`, the iterator is empty and yields nothing.

```txt
range(0, 3).for(i => print(i))   // prints 0, 1, 2
range(5, 5).for(i => print(i))   // prints nothing
range(1, 4).for(i => print(i))   // prints 1, 2, 3
```

---

### iterOf

```txt
val iterOf: (arr: Json[]) -> Iterator
```

Returns an iterator that yields each element of `arr` in order. Unlike calling `.for(f)` directly on an array, `iterOf` produces a first-class iterator value that can be passed around before consumption.

```txt
val it = iterOf([10, 20, 30])
it.for(x => print(x))   // prints 10, 20, 30
```

---

## std/number

Import:

```txt
import { parseInt32, parseFloat64 } from "std/number"
```

---

### parseInt32

```txt
val parseInt32: (s: String) -> Int32
```

Parses `s` as a base-10 integer and returns the result as an `Int32`. Leading and trailing whitespace in `s` is not trimmed — callers should `trim` if needed. If `s` cannot be parsed as a valid integer (contains non-digit characters, is empty, or the value overflows `Int32`), the behaviour is a runtime error.

Use `isInt32` to guard the call if the input is untrusted.

```txt
parseInt32("42")     // 42
parseInt32("-7")     // -7
parseInt32("0")      // 0
```

---

### parseFloat64

```txt
val parseFloat64: (s: String) -> Float64
```

Parses `s` as a base-10 floating-point number and returns the result as a `Float64`. Supports integer strings, decimal strings, and scientific notation (e.g. `"3.14"`, `"1e10"`). Leading and trailing whitespace is not trimmed. If `s` cannot be parsed, the behaviour is a runtime error.

```txt
parseFloat64("3.14")   // 3.14
parseFloat64("1e10")   // 10000000000.0
parseFloat64("42")     // 42.0
```

---

### toInt32

```txt
val toInt32: (v: Float64) -> Int32
```

Converts a `Float64` to `Int32` by truncating toward zero (dropping the fractional part). If the truncated value cannot be represented as an `Int32`, the behaviour is a runtime error.

```txt
toInt32(3.9)    // 3
toInt32(-2.1)   // -2
toInt32(0.0)    // 0
```

---

### toFloat64

```txt
val toFloat64: (v: Int32) -> Float64
```

Widens an `Int32` to `Float64`. This conversion is always exact.

```txt
toFloat64(42)    // 42.0
toFloat64(-1)    // -1.0
```

---

### isInt32

```txt
val isInt32: (s: String) -> Boolean
```

Returns `true` if `s` can be successfully parsed as an `Int32` by `parseInt32`. Use this to guard calls to `parseInt32` on untrusted input.

```txt
isInt32("42")      // true
isInt32("-7")      // true
isInt32("3.14")    // false
isInt32("hello")   // false
isInt32("")        // false
```

---

## std/result

The `Result` type is a structural convention — it has no runtime representation beyond ordinary JSON objects. No import is needed to use the shape; import from `std/result` if you want the type alias by name.

Import:

```txt
import { Result } from "std/result"
```

---

### Result

```txt
type Result<T, E> =
  | { "type": "success", "value": T }
  | { "type": "failure", "error": E }
```

`Result<T, E>` represents an operation that either succeeds with a value of type `T`, or fails with an error of type `E`. The two variants are distinguished by the `"type"` field.

Because `Result` is structural, any code that produces an object with `"type": "success"` and a `"value"` field is already compatible with `Result`. No constructor functions are required.

**Constructing results:**

```txt
val success = { "type": "success", "value": 42 }
val failure = { "type": "failure", "error": "not found" }
```

**Consuming results with pattern matching:**

```txt
val outcome = computeSomething()

match outcome
  is { "type": "success", "value": v } => print("got ${v}")
  is { "type": "failure", "error": e } => print("failed: ${e}")
```

**Chaining results:**

Because results are plain objects, you can use `map` over arrays of results or write your own combinators.

```txt
val handleResult = (r: Result<Int32, String>) =>
  match r
    is { "type": "success", "value": v } => v * 2
    is { "type": "failure", "error": _ } => 0
```

---

## std/io

Functions for reading from standard input and writing to standard output.

Import:

```txt
import { print, readLine, lines } from "std/io"
```

`print` is also available as a global without importing.

---

### print

```txt
val print: (value: Json) -> Null
```

Writes `value` to standard output followed by a newline. The value is formatted as a human-readable string: strings are printed without surrounding quotes, numbers are printed in their natural decimal form, booleans as `true`/`false`, `null` as `null`, arrays and objects as JSON.

Returns `null`.

```txt
print("hello")       // hello
print(42)            // 42
print(true)          // true
print([1, 2, 3])     // [1, 2, 3]
print({"a": 1})      // {"a": 1}
```

---

### readLine

```txt
val readLine: () -> String | Null
```

Reads one line from standard input and returns it as a string, with the trailing newline stripped. Returns `Null` when standard input is exhausted (EOF).

This is a blocking call: it waits until the user presses Enter or the input stream closes.

```txt
val name = readLine()   // blocks until Enter
match name
  is Null   => print("no input")
  else      => print("hello ${name}")
```

---

### lines

```txt
val lines: () -> Iterator
```

Returns an iterator that yields one `String` per line from standard input, with trailing newlines stripped. The iterator terminates when EOF is reached. Each step blocks until the next line is available.

The canonical form for reading stdin line-by-line:

```txt
lines().for(line =>
  val result = process(line.trim())
  print(result)
)
```

---

### readAll

```txt
val readAll: () -> String
```

Reads all of standard input and returns it as a single string. The string includes embedded newlines. This is a blocking call that returns only when EOF is reached.

```txt
val raw = readAll()
val trimmed = raw.trim()
```

---

## std/fs

Filesystem functions for reading and writing files. All functions accept paths as strings. Relative paths are resolved relative to the process working directory.

Fallible functions return `T | Error`. The `Error` value carries a message describing the OS-level failure (file not found, permission denied, etc.).

Import:

```txt
import { readFile, writeFile, readLines } from "std/fs"
```

---

### readFile

```txt
val readFile: (path: String) -> String | Error
```

Reads the entire contents of the file at `path` and returns it as a `String`. The file is read as UTF-8; a decoding error produces an `Error`.

```txt
match readFile("config.txt")
  is { "type": "success", "value": contents } => process(contents)
  is { "type": "failure", "error": e }         => print("read failed: ${e}")
```

---

### writeFile

```txt
val writeFile: (path: String, content: String) -> Null | Error
```

Writes `content` to the file at `path`, creating the file if it does not exist and replacing its contents if it does. Returns `Null` on success or an `Error` on failure.

```txt
match writeFile("out.txt", "hello\n")
  is { "type": "failure", "error": e } => print("write failed: ${e}")
  else                                  => null
```

---

### appendFile

```txt
val appendFile: (path: String, content: String) -> Null | Error
```

Appends `content` to the end of the file at `path`, creating the file if it does not exist. Returns `Null` on success or an `Error` on failure.

```txt
appendFile("log.txt", "[INFO] started\n")
```

---

### readLines

```txt
val readLines: (path: String) -> Iterator | Error
```

Opens the file at `path` and returns an iterator that yields one `String` per line, with trailing newlines stripped. Returns an `Error` immediately if the file cannot be opened. Lines are read lazily — each step reads the next line from the OS buffer.

The iterator must be fully consumed or the underlying file handle will not be closed until the program exits.

```txt
match readLines("data.csv")
  is { "type": "failure", "error": e } => print("cannot open: ${e}")
  is { "type": "success", "value": it } =>
    it.for(line => process(line))
```

---

### readJson

```txt
val readJson: (path: String) -> Json | Error
```

Reads the file at `path` and parses its contents as JSON. Returns the parsed `Json` value on success, or an `Error` if the file cannot be read or the contents are not valid JSON.

```txt
match readJson("config.json")
  is { "type": "success", "value": cfg } => run(cfg)
  is { "type": "failure", "error": e }   => print("bad config: ${e}")
```

---

### writeJson

```txt
val writeJson: (path: String, value: Json) -> Null | Error
```

Serialises `value` to a JSON string and writes it to the file at `path`, replacing any existing contents. The output is compact (no added whitespace). Returns `Null` on success or an `Error` on failure.

```txt
val record = { "name": "Alice", "score": 99 }
writeJson("result.json", record)
```

---

### exists

```txt
val exists: (path: String) -> Boolean
```

Returns `true` if a file or directory exists at `path`, `false` otherwise. Does not distinguish between files and directories; use `readFile` or other operations to detect the kind.

```txt
if exists("config.json")
  then readJson("config.json")
  else { "debug": false }
```

---

## std/http

HTTP client functions for making outbound requests. All functions are synchronous and blocking. Use `async` at the call site when concurrency is needed.

Fallible functions return `T | Error`. The `Error` value carries a message describing the failure (network error, DNS failure, TLS error, etc.). HTTP error status codes (4xx, 5xx) are **not** errors — they are returned as `HttpResponse` values with the appropriate `"status"` field. Only transport-level failures produce `Error`.

Import:

```txt
import { fetch, fetchJson, postJson } from "std/http"
```

### Types

```txt
type HttpResponse = {
  "status": Int32,
  "headers": { ...String },
  "body": String
}

type HttpOptions = {
  "method": String,
  "headers": { ...String },
  "body": String
}
```

`HttpResponse` and `HttpOptions` are structural types. All fields of `HttpOptions` are optional at the call site — omitted fields fall back to defaults (`"GET"`, empty headers, empty body).

---

### fetch

```txt
val fetch: (url: String) -> HttpResponse | Error
```

Sends a GET request to `url` and returns the response. On a transport-level failure, returns an `Error`.

HTTP error status codes are returned as successful `HttpResponse` values; callers must inspect `"status"` to detect application-level errors.

```txt
match fetch("https://api.example.com/ping")
  is { "type": "failure", "error": e }      => print("network error: ${e}")
  is { "type": "success", "value": resp } =>
    if resp["status"] == 200
      then print("ok")
      else print("unexpected status: ${resp["status"]}")
```

---

### fetchWith

```txt
val fetchWith: (url: String, options: HttpOptions) -> HttpResponse | Error
```

Sends a request to `url` using the method, headers, and body specified in `options`. Use this for non-GET methods, custom headers, or request bodies. Fields not provided in `options` use defaults.

```txt
val resp = fetchWith("https://api.example.com/items", {
  "method": "DELETE",
  "headers": { "Authorization": "Bearer ${token}" },
  "body": ""
})
```

---

### fetchJson

```txt
val fetchJson: (url: String) -> Json | Error
```

Sends a GET request to `url`, parses the response body as JSON, and returns the parsed value. Returns an `Error` if there is a transport failure, the response status is not 2xx, or the body is not valid JSON.

This is the idiomatic function for consuming JSON APIs:

```txt
match fetchJson("https://api.example.com/users")
  is { "type": "success", "value": users } =>
    users.map(u => u["name"]).for(name => print(name))
  is { "type": "failure", "error": e }     =>
    print("failed: ${e}")
```

Concurrent requests use `async` at the call site:

```txt
val [users, posts] = await([
  async(() => fetchJson("https://api.example.com/users")),
  async(() => fetchJson("https://api.example.com/posts"))
])
```

---

### postJson

```txt
val postJson: (url: String, body: Json) -> HttpResponse | Error
```

Serialises `body` as JSON and sends it as a POST request to `url` with `Content-Type: application/json`. Returns the full `HttpResponse` (rather than a parsed body) so the caller can inspect the status code and headers. Returns an `Error` on transport failure.

```txt
val resp = postJson("https://api.example.com/users", {
  "name": "Alice",
  "email": "alice@example.com"
})

match resp
  is { "type": "success", "value": r } =>
    print("created with status ${r["status"]}")
  is { "type": "failure", "error": e } =>
    print("post failed: ${e}")
```

---

## std/server

HTTP server functions. The module provides two modes with the same handler signature:

- **`serve`** — single-threaded, sequential. The handler processes one request at a time. `var` state is safe without any locking. The natural choice for most servers.
- **`threadPool.serve`** — multi-threaded. `ThreadPool` is the existing concurrency primitive from §32.5; `.serve` is an additional method on it. The handler is called concurrently on the pool's threads. The same `var`-capture restriction as `pool.async` applies — the handler must not close over `var` bindings.

Both forms block the calling thread. Wrap in `async` if the server should run in the background alongside other work.

Import:

```txt
import { serve, json, notFound, pathMatch } from "std/server"
```

### Types

```txt
type HttpRequest = {
  "method":  String,
  "path":    String,
  "query":   { ...String },
  "headers": { ...String },
  "body":    String
}
```

`HttpResponse` is the same type as in `std/http`:

```txt
type HttpResponse = {
  "status":  Int32,
  "headers": { ...String },
  "body":    String
}
```

---

### serve

```txt
val serve: (port: Int32, handler: (HttpRequest) -> HttpResponse) -> Null
```

Starts an HTTP server on `port` and calls `handler` for each incoming request, **sequentially** — one request at a time. Blocks the calling thread indefinitely.

Because the handler runs on a single thread, closing over `var` bindings is safe.

```txt
serve(3000, req =>
  { "status": 200, "headers": {}, "body": "hello" }
)
```

Routing with pattern matching:

```txt
import { serve, json, notFound } from "std/server"

serve(3000, req =>
  match req
    has { "method": "GET",  "path": "/users" } => json(200, getUsers())
    has { "method": "POST", "path": "/users" } =>
      match parseBody(req)
        is { "type": "failure", "error": e }    => badRequest(e)
        is { "type": "success", "value": body } => createUser(body)
    else => notFound()
)
```

Stateful server — `var` is safe because `serve` is sequential:

```txt
var store = []

serve(3000, req =>
  match req
    has { "method": "GET",  "path": "/items" } => json(200, store)
    has { "method": "POST", "path": "/items" } =>
      match parseBody(req)
        is { "type": "failure", "error": e }    => badRequest(e)
        is { "type": "success", "value": item } =>
          push(store, item)
          json(201, item)
    else => notFound()
)
```

To run the server non-blocking alongside other work:

```txt
val server = async(() => serve(3000, handler))
doOtherWork()
await(server)
```

---

### ThreadPool.serve

```txt
// called as: pool.serve(port, handler)
// pool is a ThreadPool constructed with threadPool(n)
```

Starts an HTTP server on `port` and dispatches each incoming request to the pool's threads. Multiple requests are handled concurrently up to the pool's thread count. Blocks the calling thread indefinitely.

Because handlers run concurrently, the **same `var`-capture restriction as `pool.async` applies** — the handler must not close over `var` bindings. This is a compile-time error where statically detectable.

```txt
threadPool(8).serve(3000, req =>
  match req
    has { "method": "GET", "path": "/users" } => json(200, getUsers())
    else                                       => notFound()
)
```

The handler may use `async` internally for per-request fan-out:

```txt
threadPool(8).serve(3000, req =>
  val [users, posts] = await([
    async(() => fetchJson("https://db/users")),
    async(() => fetchJson("https://db/posts"))
  ])
  json(200, { "users": users, "posts": posts })
)
```

---

### json (helper) {#json-helper}

```txt
val json: (status: Int32, body: Json) -> HttpResponse
```

Builds an `HttpResponse` with the given status, the JSON serialisation of `body` as the response body, and `Content-Type: application/json` set automatically.

```txt
json(200, { "users": ["Alice", "Bob"] })
json(201, { "id": 42 })
json(404, { "error": "not found" })
```

---

### text (helper) {#text-helper}

```txt
val text: (status: Int32, body: String) -> HttpResponse
```

Builds an `HttpResponse` with the given status, `body` as the response body, and `Content-Type: text/plain`.

```txt
text(200, "pong")
```

---

### redirect

```txt
val redirect: (url: String) -> HttpResponse
```

Builds a 302 response with a `Location` header pointing to `url` and an empty body.

```txt
redirect("/login")
redirect("https://example.com/new-path")
```

---

### notFound

```txt
val notFound: () -> HttpResponse
```

Builds a 404 response with a plain-text body of `"Not Found"`.

```txt
else => notFound()
```

---

### badRequest

```txt
val badRequest: (message: String) -> HttpResponse
```

Builds a 400 response with `message` as the plain-text body.

```txt
badRequest("missing required field: name")
```

---

### pathMatch

```txt
val pathMatch: (pattern: String, path: String) -> { ...String } | Null
```

Matches `path` against `pattern`. Returns an object of captured named parameters if the match succeeds, or `Null` if it does not. Pattern segments beginning with `:` are named captures; all other segments must match literally.

```txt
pathMatch("/users/:id",         "/users/42")       // { "id": "42" }
pathMatch("/users/:id/posts",   "/users/42/posts") // { "id": "42" }
pathMatch("/users/:id",         "/items/42")       // null
pathMatch("/static",            "/static")         // {}
```

Typical usage inside a handler:

```txt
val params = pathMatch("/users/:id", req["path"])
match params
  is Null => notFound()
  else    => json(200, getUser(params["id"]))
```

---

### parseBody

```txt
val parseBody: (req: HttpRequest) -> Json | Error
```

Parses `req["body"]` as JSON. Returns the parsed value on success, or an `Error` if the body is not valid JSON. Equivalent to calling `parseJson(req["body"])` from `std/http` but reads more naturally inside a handler.

```txt
match parseBody(req)
  is { "type": "failure", "error": e }    => badRequest(e)
  is { "type": "success", "value": body } => createItem(body)
```

---

## std/template

Functions for rendering template strings. Template syntax uses `${key}` holes where `key` is either a top-level field name or a dot-separated path into the data record. Everything outside a `${}` hole is emitted verbatim, including newlines.

Import:

```txt
import { render, renderWith } from "std/template"
```

---

### render

```txt
val render: (path: String, data: {}) -> String | Error
```

Reads the file at `path` and renders it as a template against `data`. Returns the rendered `String` on success, or an `Error` if the file cannot be read. Intended for use with `.lint` template files.

```txt
// greet.lint:
// Hello, ${name}! Your score is ${stats.score}.

import { render } from "std/template"

match render("greet.lint", { "name": "Alice", "stats": { "score": 42 } })
  is { "type": "error", "message": e } => print("error: ${e}")
  else => val output = render("greet.lint", { "name": "Alice", "stats": { "score": 42 } })
          print(output)
```

---

### renderWith

```txt
val renderWith: (template: String, data: {}) -> String
```

Renders a template string directly against `data`. Holes are `${key}` where `key` is a field name or dot-separated path. Missing keys render as `"null"`.

```txt
renderWith("Hello, ${name}!", { "name": "Alice" })
// "Hello, Alice!"

renderWith("${user.name} scored ${user.score}", { "user": { "name": "Bob", "score": 99 } })
// "Bob scored 99"

renderWith("x = ${x}, y = ${y}", { "x": 1, "y": 2 })
// "x = 1, y = 2"
```

---

## std/test

A lightweight test framework. Tests are plain Lin values — `test` returns a `Test` record, `suite` groups them, `run` executes and reports. A test file is a regular `.lin` file run with `lin run`; no special runner binary is required.

Import:

```txt
import { suite, test, run, expect } from "std/test"
```

**Conventions:** name test files `*.test.lin` or `*_test.lin`. A future `lin test` subcommand will discover and run them automatically; until then `lin run my_test.lin` works.

**Basic usage:**

```txt
import { suite, test, run, expect } from "std/test"

val add = (a: Int32, b: Int32): Int32 => a + b

val arithmetic = suite("arithmetic", [
  test("adds two positives", () =>
    expect(add(1, 2)).toBe(3)
  ),
  test("adds negatives", () =>
    expect(add(-1, -1)).toBe(-2)
  ),
  test("multiple assertions", () =>
    expect(add(0, 0)).toBe(0)
    expect(add(10, -10)).toBe(0)
  )
])

run([arithmetic])
```

**Multi-assertion tests** use bare expression statements in the lambda body. Each `expect(...).toX()` call is evaluated in order; the test collects all failures before reporting rather than stopping at the first.

---

### Types

```txt
type Assertion =
  | { "type": "pass" }
  | { "type": "fail", "message": String }

type Test = {
  "name": String,
  "run": () -> Assertion | Assertion[]
}

type Suite = {
  "name": String,
  "tests": Test[]
}

type Asserter = {
  "value": Json,
  "toBe":       (Json) -> Assertion,
  "toBeNull":   () -> Assertion,
  "toSatisfy":  ((Json) -> Boolean) -> Assertion,
  "toSucceed":  () -> Assertion,
  "toFail":     () -> Assertion,
  "toFailWith": (String) -> Assertion
}
```

`Assertion`, `Test`, `Suite`, and `Asserter` are structural — they have no special runtime representation. The type names are exported for documentation purposes only; you do not need to import them to use the framework.

---

### suite

```txt
val suite: (name: String, tests: Test[]) -> Suite
```

Groups a list of `Test` values under a name. The name is printed as a heading when `run` executes the suite.

```txt
val myTests = suite("math", [
  test("one plus one", () => expect(1 + 1).toBe(2))
])
```

Suites are plain data and can be composed, filtered, or built programmatically:

```txt
val cases = range(1, 5).map(n =>
  test("double of ${toString(n)}", () =>
    expect(n * 2).toBe(n + n)
  )
)
val generated = suite("doubles", cases)
```

---

### test

```txt
val test: (name: String, body: () -> Assertion | Assertion[]) -> Test
```

Declares a single test case. `body` is a zero-argument lambda that returns either one `Assertion` or an array of `Assertion` values. All assertions in the body are evaluated; the test fails if any of them fail.

**Single assertion:**

```txt
test("empty array has length zero", () =>
  expect(length([])).toBe(0)
)
```

**Multiple assertions** — write bare expression statements; each is evaluated in order:

```txt
test("string conversions", () =>
  expect(toString(42)).toBe("42")
  expect(toString(true)).toBe("true")
  expect(toString(null)).toBe("null")
)
```

**Using `val` bindings inside a test:**

```txt
test("pipeline result", () =>
  val xs = [1, 2, 3].map(x => x * 2)
  expect(xs[0]).toBe(2)
  expect(length(xs)).toBe(3)
)
```

---

### run

```txt
val run: (suites: Suite[]) -> Null
```

Executes all suites in order, prints a summary to stdout, and exits the process with a non-zero code if any test failed. Each passing test prints a line beginning with `ok`; each failing test prints the failure message with the test name. A final line reports total passed and failed counts.

```txt
run([unitTests, integrationTests])
```

Output format (illustrative):

```txt
arithmetic
  ok  adds two positives
  ok  adds negatives
  FAIL  identity element
    expected: 1
    actual:   0

1 failed, 2 passed
```

`run` always executes all tests — it does not short-circuit on the first failure.

---

### expect

```txt
val expect: (value: Json) -> Asserter
```

Wraps `value` in an `Asserter`. Call one assertion method on the returned object to produce an `Assertion`.

```txt
expect(add(1, 2)).toBe(3)
expect(result).toSucceed()
expect(name).toSatisfy(s => length(s) > 0)
```

#### .toBe

```txt
.toBe: (expected: Json) -> Assertion
```

Passes when `value` is deeply structurally equal to `expected` (same semantics as `==`). Works for all JSON-compatible values. Object comparison is order-independent; array comparison is ordered.

```txt
expect([1, 2, 3]).toBe([1, 2, 3])    // pass
expect({ "a": 1 }).toBe({ "a": 1 }) // pass
expect([1, 2]).toBe([2, 1])          // fail
```

Failure message: `expected: <expected>\nactual:   <actual>`

#### .toBeNull

```txt
.toBeNull: () -> Assertion
```

Passes when `value` is `null`.

```txt
expect(obj["missing"]).toBeNull()
```

#### .toSatisfy

```txt
.toSatisfy: (pred: (Json) -> Boolean) -> Assertion
```

Passes when `pred(value)` returns `true`. The general escape hatch for structural assertions that cannot be expressed with `.toBe`, including inspecting union shapes:

```txt
expect(name).toSatisfy(s => length(s) > 0)
expect(result).toSatisfy(r => r has { "type": "success" })
```

Failure message: `value did not satisfy predicate: <value>`

#### .toSucceed

```txt
.toSucceed: () -> Assertion
```

Passes when `value` has shape `{ "type": "success", ... }`.

```txt
expect(parseAge("42")).toSucceed()
```

Failure message: `expected success, got: <value>`

#### .toFail

```txt
.toFail: () -> Assertion
```

Passes when `value` has shape `{ "type": "failure", ... }`.

```txt
expect(parseAge("not-a-number")).toFail()
```

Failure message: `expected failure, got: <value>`

#### .toFailWith

```txt
.toFailWith: (message: String) -> Assertion
```

Passes when `value` has shape `{ "type": "failure", "error": e }` and `e == message`.

```txt
expect(divide(1.0, 0.0)).toFailWith("Cannot divide by zero")
```

Failure message: `expected failure with "${message}", got: <value>`
