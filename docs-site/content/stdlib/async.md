# std/async

Concurrency primitives: async/await, workers, thread pools.

```lin
import { async, await, parallel, race, timeout, retry } from "std/async"
import { worker, message, request, close } from "std/async"
import { threadPool } from "std/async"
```

## Function reference

| Function | Signature | Description |
| --- | --- | --- |
| `async` | `(() -> T) -> Promise` | Run thunk on background thread |
| `await` | `(Promise) -> T` | Block until promise resolves |
| `close` | `(Worker) -> Null` | Shut down worker |
| `message` | `(Worker, Msg) -> Null` | Fire-and-forget message to worker |
| `parallel` | `((() -> T)[]) -> T[]` | Run array of thunks concurrently |
| `race` | `(Promise[]) -> T` | First promise to complete wins |
| `request` | `(Worker, Msg) -> Reply` | Synchronous request/reply to worker |
| `retry` | `(() -> T, Int32) -> T` | Retry thunk up to n times |
| `threadPool` | `(Int32) -> ThreadPool` | Create thread pool |
| `timeout` | `(Promise, Int32) -> T` | Add timeout to a promise |
| `worker` | `((Msg) -> Reply, () -> Null) -> Worker` | Create background worker |

---

### `async` / `await`

```lin
val p = async(() => expensiveComputation())
// ... do other work ...
val result = await(p)
```

The thunk may not capture `var` bindings (compile-time error where detectable).

---

### `parallel`

```lin
val [a, b, c] = parallel([
  () => fetchUsers(),
  () => fetchPosts(),
  () => fetchComments()
])
```

---

### `race`

```lin
val fastest = await(race([
  async(() => fetchFrom("mirror-a")),
  async(() => fetchFrom("mirror-b"))
]))
```

---

### `timeout`

```lin
val result = await(timeout(longOp, 5000))
match result
  is Null  => print("timed out")
  is Error => print("failed")
  else     => print("ok: ${result}")
```

---

### `retry`

```lin
val data = await(retry(() => unstableFetch(), 3))
```

---

### `worker`

```lin
val w = worker(
  (msg: String) => "echo: ${msg}",
  () => null
)

val reply = request(w, "hello")   // "echo: hello"
message(w, "fire-and-forget")
close(w)
```

Workers may close over `var` bindings (single-threaded, no races).

---

### `threadPool`

```lin
val pool = threadPool(8)
val p = pool.async(() => heavyWork())
val result = await(p)

// Multiple tasks:
val results = await(pool.async([
  () => work(1),
  () => work(2),
  () => work(3)
]))

// HTTP server on pool:
import { serve, json } from "std/http"
pool.serve(3000, req => json(200, { "status": "ok" }))
```
