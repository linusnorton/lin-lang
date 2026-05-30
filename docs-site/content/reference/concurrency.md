# Concurrency Reference

Import concurrency primitives from `std/async`:

```lin
import { async, await, parallel, race, timeout, retry, worker, message, request, close, threadPool } from "std/async"
```

## `Promise<T>`

An opaque runtime type representing a value of type `T` being computed on another OS thread.

`T` must be a **transferable** type: JSON-compatible values. The opaque types (`Function`, `Iterator`, `Worker`, `ThreadPool`, `Promise`) are not transferable.

## `async(thunk)`

Spawns the thunk on a new OS thread, returns a `Promise<T | Error>` immediately:

```lin
val p: Json = async(() => compute())
```

The thunk must be `() => T` (zero arguments). Thunks may not capture `var` bindings — compile-time error where detectable.

## `await(promise)`

Blocks the current thread until the promise resolves:

```lin
val result = await(p)
```

Runtime errors inside the thunk are caught at the thread boundary and surface as `Error` values at `await`.

`await` also accepts a `Promise[]` and returns a result array:

```lin
val [a, b] = await([asyncA, asyncB])
```

## `parallel(thunks)`

Runs all thunks concurrently and returns results in input order:

```lin
val [users, posts] = parallel([
  () => fetchUsers(),
  () => fetchPosts()
])
```

Same var-capture restriction as `async`.

## `race(promises)`

Resolves with the first promise to complete:

```lin
val fastest = race([mirror1Promise, mirror2Promise])
val data = await(fastest)
```

## `timeout(promise, ms)`

Resolves with the original value if completed within `ms` milliseconds, otherwise resolves to `Null`:

```lin
val result = await(timeout(longOp, 5000))
match result
  is Null  => print("timed out")
  is Error => print("failed")
  else     => print("got ${result}")
```

## `retry(thunk, n)`

Runs the thunk up to `n` times, returning the first non-Error result:

```lin
val data = await(retry(() => unstableNetwork(), 3))
```

## `Worker<Msg, Reply>`

A long-lived OS thread processing messages sequentially.

### `worker(handler, onClose)`

Create a worker:

```lin
val w = worker(
  (msg: String) => "echo: ${msg}",
  () => null
)
```

### `request(worker, msg)`

Send a message, wait for the reply:

```lin
val reply = request(w, "hello")
// or: val reply = w.request("hello")
```

### `message(worker, msg)`

Fire-and-forget — enqueues without waiting:

```lin
message(w, "background task")
```

### `close(worker)`

Waits for in-progress message to finish, calls `onClose`, terminates the thread:

```lin
close(w)
```

Workers may close over `var` bindings (safe because messages are processed one at a time).

## `ThreadPool`

### `threadPool(n)`

Create a pool of `n` threads:

```lin
val pool = threadPool(8)
```

### `pool.async(thunk)`

Submit a single thunk to the pool:

```lin
val p = pool.async(() => work())
```

### `pool.async(thunks)`

Submit multiple thunks:

```lin
val results = await(pool.async([() => work(1), () => work(2)]))
```

### `pool.serve(port, handler)`

Multi-threaded HTTP server dispatching to pool threads:

```lin
import { json } from "std/http"

pool.serve(3000, req => json(200, { "status": "ok" }))
```

## Transferability rules

A value is transferable if it is:
- A JSON-compatible value: `String`, `Boolean`, `Null`, any numeric, `T[]` of transferable `T`, or an object with transferable values.
- A `Function` that closes over no `var` bindings.

Non-transferable: `Function` with `var` captures, `Iterator`, `Worker`, `ThreadPool`, `Promise`.
