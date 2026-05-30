# std/time

Timestamps, timing, and formatting. All timestamps are Unix time in milliseconds.

```lin
import { now, sleep, toIso, fromIso, format, parse, startTimer, elapsed } from "std/time"
```

`Timer` is an opaque runtime type returned by `startTimer`.

## Function reference

| Function | Signature | Description |
| --- | --- | --- |
| `elapsed` | `(Timer) -> Int64` | Milliseconds since timer was started |
| `format` | `(Int64, String) -> String` | Format timestamp with strftime pattern |
| `fromIso` | `(String) -> Int64 \| Error` | Parse ISO 8601 string to ms timestamp |
| `now` | `() -> Int64` | Current Unix timestamp in milliseconds |
| `parse` | `(String, String) -> Int64 \| Error` | Parse date string with format pattern |
| `sleep` | `(Int32) -> Null` | Block for n milliseconds |
| `startTimer` | `() -> Timer` | Start high-resolution elapsed timer |
| `toIso` | `(Int64) -> String` | Format timestamp as ISO 8601 |

---

### `now`

```lin
val ts = now()   // e.g. 1716825600000
```

---

### `sleep`

```lin
sleep(1000)   // wait 1 second
```

---

### `toIso` / `fromIso`

```lin
toIso(0)       // "1970-01-01T00:00:00.000Z"
toIso(now())   // e.g. "2025-05-27T14:32:07.123Z"

fromIso("2024-01-15T10:30:00Z")   // 1705313400000
fromIso("2024-01-15")             // 1705276800000
fromIso("bad")                    // { "type": "failure", "error": "..." }
```

---

### `format` / `parse`

Uses strftime-style patterns:

```lin
format(now(), "%Y-%m-%d")           // "2025-05-27"
format(now(), "%H:%M:%S")           // "14:32:07"
format(now(), "%Y-%m-%dT%H:%M:%S")  // "2025-05-27T14:32:07"

parse("2024-01-15", "%Y-%m-%d")   // 1705276800000
```

---

### `startTimer` / `elapsed`

For high-resolution timing of code sections:

```lin
val t = startTimer()
doHeavyWork()
print("took ${elapsed(t)}ms")
```

---

### Measuring elapsed time without a timer

```lin
val start = now()
doWork()
val duration = now() - start
print("elapsed: ${duration}ms")
```
