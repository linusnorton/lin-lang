# FFI

Call C functions from Lin via `import foreign`. This example links against a
small C static library (`mathlib.c`) and calls into it.

## What it demonstrates

- `import foreign "<path-to-.a>"` to declare external C symbols with Lin types.
- Typed foreign signatures: `add: (Int32, Int32) => Int32`, `square: (Float64) => Float64`.
- The compiler emits LLVM `declare`s and passes the library path to the linker.

## Structure

- **`main.lin`** — declares the two foreign functions and calls them.
- **`mathlib.c`** — the C source for `add` and `square`.

## Build / Run

FFI works only via `lin build` (not `lin run`): the C library must be compiled
and linked. Build the static library first, then build and run:

```bash
cc -c examples/ffi/mathlib.c -o examples/ffi/mathlib.o
ar rcs examples/ffi/libmathlib.a examples/ffi/mathlib.o
lin build examples/ffi/main.lin -o ffi && ./ffi
```

Type-check only (no `.a` needed):

```bash
lin check examples/ffi/main.lin
```

CI does not run this through the plain `lin run` loop (it needs the prebuilt
`.a`); the authoritative end-to-end FFI check lives in the Rust integration suite
(`test_ffi_end_to_end_c_library`), which compiles the C and links it. See
`crates/lin-runtime/lin.h` for the C/C++ interop header.
