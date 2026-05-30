# Getting Started

This guide walks you through installing Lin, writing your first program, and getting oriented in the language.

## Installation

### Download a prebuilt binary

Download the latest release for your platform from the [GitHub releases page](https://github.com/linusnorton/lin-lang/releases). Extract the archive and place the `lin` binary somewhere on your `PATH`:

```
tar xzf lin-linux-x86_64.tar.gz
sudo mv lin /usr/local/bin/
lin --version
```

### Build from source

You need a Rust toolchain (stable) and LLVM 22.

```
git clone https://github.com/linusnorton/lin-lang.git
cd lin-lang
LLVM_SYS_220_PREFIX=/usr/lib/llvm-22 cargo build --workspace
# The binary is at target/debug/lin
```

## Your first program

Create a file called `hello.lin`:

```lin
import { print } from "std/io"

print("hello, world!")
```

Compile and run it:

```
lin build hello.lin -o hello
./hello
```

Output:

```
hello, world!
```

## String interpolation

Lin uses `${expr}` for string interpolation — the only way to build strings from parts:

```lin
import { print } from "std/io"

val name = "world"
val n = 42
print("hello, ${name}! The answer is ${n}.")
```

## Reading user input

```lin
import { print, readLine } from "std/io"

print("What is your name?")
val name = readLine()
match name
  is Null => print("no input")
  else    => print("Hello, ${name}!")
```

## What's next?

Work through the tutorials in order to learn the language properly:

1. [Hello World & I/O](/tutorials/01-hello-world.html) — more about I/O
2. [Values & Types](/tutorials/02-values-and-types.html) — the type system
3. [Functions](/tutorials/03-functions.html) — functions and closures
4. [Working with JSON](/tutorials/04-json-data.html) — objects and arrays
5. [Pattern Matching](/tutorials/05-pattern-matching.html) — match and is/has
6. [Arrays & Iteration](/tutorials/06-arrays-and-iteration.html) — map/filter/reduce
7. [Modules](/tutorials/07-modules.html) — imports and exports
8. [Error Handling](/tutorials/08-error-handling.html) — no exceptions
9. [Concurrency](/tutorials/09-concurrency.html) — async and workers
