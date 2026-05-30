# Types Reference

## Primitive types

| Type | Description | Example literal |
| --- | --- | --- |
| `String` | UTF-8 text | `"hello"` |
| `Boolean` | Truth value | `true` / `false` |
| `Null` | Absence of value | `null` |
| `Int8` | 8-bit signed integer | `42i8` |
| `Int16` | 16-bit signed integer | `1000i16` |
| `Int32` | 32-bit signed integer (default) | `42` |
| `Int64` | 64-bit signed integer | `42i64` |
| `UInt8`–`UInt64` | Unsigned integer families | `255u8` |
| `Float32` | 32-bit IEEE 754 float | `3.14f32` |
| `Float64` | 64-bit IEEE 754 float (default) | `3.14` |

Without a suffix, integer literals default to `Int32` and floating-point literals default to `Float64`.

## `Number`

`Number` is a built-in union alias covering every numeric type family. It has no runtime representation of its own — every value retains its specific numeric type:

```lin
type Number =
  | Int8 | Int16 | Int32 | Int64
  | UInt8 | UInt16 | UInt32 | UInt64
  | Float8 | Float16 | Float32 | Float64
```

## `Json`

`Json` is the recursive union of all JSON-compatible values:

```lin
type Json =
  | String | Boolean | Null | Number
  | Json[]
  | { ...Json }   // any object whose values are Json
```

Use `Json` when the shape of data is not statically known.

## Union types

```lin
val x: String | Null = null
val id: String | Int32 = "user-42"

type Result<T, E> =
  | { "type": "success", "value": T }
  | { "type": "failure", "error": E }
```

Union types use `|`. The type `T | Null` is the common pattern for optional values.

## Object types

```lin
type Person = {
  "name": String,
  "age": Int32
}
```

Object types are structural. A value with additional fields is compatible with a smaller structural type.

## Array types

`T[]` — unbounded array of `T`:

```lin
val names: String[] = ["Alice", "Bob"]
```

`[T1, T2, T3]` — fixed-length array with specified element types:

```lin
val pair: [String, Int32] = ["age", 42]
```

## Function types

```lin
type Predicate<T> = (T) => Boolean
type Mapper<T, U> = (T) => U
```

## Generic types

```lin
type Box<T> = {
  "value": T,
  "label": String
}
```

Generic types are covariant in producer positions and contravariant in consumer positions.

## Opaque runtime types

| Type | Description |
| --- | --- |
| `Iterator<T>` | Stateful traversal producing `T` |
| `Iterable<T>` | Any value that can produce `Iterator<T>` |
| `Promise<T>` | Value being computed on another thread |
| `Worker<Msg, Reply>` | Long-lived background thread |
| `ThreadPool` | Fixed-size thread pool |
| `Function` | Opaque function reference |

These types are not JSON values and cannot be stored in JSON objects or arrays.

## Structural typing

Types are structural by default. Two types are compatible if they describe the same shape:

```lin
type Named = { "name": String }

val greet = (x: Named): String => "Hello ${x["name"]}"

// Works — the value has at least the "name" field
greet({ "name": "Alice", "age": 30 })
```

## Numeric widening

Numeric types widen automatically in arithmetic and comparison. The widened type is the smallest type that can fully represent both operands. Explicit narrowing uses stdlib functions (`toInt32`, `toFloat64`, etc.) and may fail at runtime if the value cannot be represented.
