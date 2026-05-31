# Processes

Spawn a subprocess, read its stdout into a byte buffer, and wait for its exit
code. A small tour of `std/proc` over a real child process.

## What it demonstrates

- `std/proc`: `spawn(argv)`, `readStdout(handle, buf)`, `wait(handle)`.
- Flat `UInt8[]` byte buffers as read targets (`readStdout` fills the buffer in place).
- Reading a child's bytes and propagating its exit code.

## Structure

- **`main.lin`** — spawns `sh -c "printf 'Hi'"`, reads its stdout, prints the bytes and exit code.
- **`proc.test.lin`** — asserts the read byte count, the actual bytes (`'H'`=72, `'i'`=105),
  and a non-zero exit code from `sh -c "exit 3"`.

## Typing note

The process handle returned by `spawn` is an **opaque** runtime value, so it is
not given a record type. What is precise is typed: the stdout buffer is a flat
`UInt8[]`, and argv is `String[]`. (The lower-level spawn/exit-code assertions
also live in the Rust integration suite.)

## Run / Test

```bash
lin run examples/processes/main.lin      # spawn, read bytes, print exit code
lin test examples/processes/             # the recoverable, deterministic assertions
```
