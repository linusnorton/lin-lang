# std/test — utility gaps & proposals

Findings collected while restructuring `examples/*` into per-module unit suites +
`integration.test.lin` (the convention: a `<module>.test.lin` per source module, one
`integration.test.lin` per project). The current `std/test` API is intentionally tiny:

```
expect(v)                       -> asserter
  .toBe(expected)               structural equality
  .toBeNull()
  .toSatisfy(pred)              pred(value) is true
  .toSucceed() / .toFail()      tagged { "type": "success" | "failure" }
  .toFailWith(message)          failure whose "error" == message
test(name, () => Assertion[])   a test body returns an array of assertions
suite(name, Json[])             group of tests
run(suite)                      execute + report, exit 1 on any failure
```

Every gap below was hit repeatedly across ≥2 projects (calc, codec, config, dijkstra,
matrix, report, web-server, raspberry-controller, concurrency, result, processes).

## 1. Import stubbing / mocking (the headline request)

**Problem.** A module that calls a dependency cannot be unit-tested in true isolation:
`report.lin`'s `validRecords`/`stats` necessarily run `parse.lin`'s `parseRow`;
`config/loader.lin`'s `load` always runs the real `schema.lin` validators;
`web-server/handlers.lin` calls `render()` which reads a real `.lint` off disk. Today we
sidestep this by feeding fixed inputs and pushing I/O-bound code to integration — but the
"unit" suites are really small-integration suites.

**Why it's hard in Lin.** Imports resolve to mangled LLVM symbols at compile time
(`std_array_map`, `._report_parseRow`); there is no runtime indirection to swap, and `val`
bindings to top-level functions are forward-declared and direct-called. So a JS-style
`mock("./parse", {...})` would require either (a) a compile-time test mode that rebinds an
import to a provided stub module, or (b) making imported functions injectable.

**Proposals (in order of effort):**
- **(a) Dependency injection as the idiom (no compiler change).** Encourage modules to take
  their collaborators as `Function` parameters with a default of the real impl, so tests pass
  a stub. Blocked today by **no default *function* arguments / no first-class partial of the
  real impl as a default** — and the closure-ABI sharp edges we already hit. Lowest-magic, but
  needs ergonomic defaults. *Recommend documenting this pattern once default-fn-args are solid.*
- **(b) `lin test`-level module substitution.** A test could declare
  `mock "./parse" with "./parse_stub"` and the test runner compiles the suite with that import
  redirected. Self-contained, no runtime cost, matches Lin's compile-time model. Most useful;
  medium compiler work in `lin-compile`'s import resolution.
- **(c) A `std/test` spy/stub built on a mutable indirection cell.** Only works for collaborators
  that are *already* passed as values; degenerates into (a).

**Recommendation:** pursue (b) as the real "mocking" story; meanwhile adopt (a) where
default-fn-args allow, and keep pure logic separate from I/O so most units need no mocks.

## 2. Float-tolerance matcher `toBeCloseTo(expected, epsilon)`

`toBe` is exact. f32/f64 pipelines can't be asserted directly: raspberry-controller's
`encodePacket`→`parsePacket` round-trips `0.1` back as `0.10000000149011612`; matrix/result
assert through `toFixed(n)` string formatting as a workaround. A `toBeCloseTo` (abs/rel
epsilon) matcher is the single most-requested addition for numeric code.

## 3. Collection / string matchers

Recurring `expect(xs.length(...)).toBe(n)` and `expect(s.contains(x)).toBe(true)` boilerplate.
Add: `toHaveLength(n)`, `toContain(item)` (array membership), `toContainString(substr)` /
`toContainKey(k)`. web-server asserts on serialized JSON via `.contains("\"status\": \"ok\"")`
— brittle to spacing; a JSON-aware `toMatchObject(partial)` (structural partial match) would be
far more robust than substring checks.

## 4. Table-driven tests `test.each`

Many suites are N near-identical single-assertion cases differing only by input/expected
(operator-by-operator in calc/codec, rejection-by-rejection in validate, row/col in matrix,
load success/failure pairs in config). A `testEach(name, rows, (row) => Assertion[])` combinator
would collapse a lot of copy-paste and make the cases data.

## 5. Setup / shared fixtures

Fixtures are rebuilt inline per test (`buildAdj([...])`, `applyDefaults({...})`, web-server's
`makeReq(...)`), and a `sample` batch is duplicated between a project's unit and integration
suites. A per-suite fixture binding or `beforeEach`-style hook would remove duplication. (Lin's
value model makes a simple "fixture thunk evaluated per test" version easy.)

## 6. Async test helpers

Concurrency suites await/block synchronously inside the test thunk (works), but there's no
async-aware assertion (assert a promise resolves/rejects; a timeout-bounded assertion). Time-
dependent tests rely on hardcoded sleep margins. Lower priority; revisit if more async examples
land.

## Suggested first slice
`toBeCloseTo`, `toContain`/`toContainString`/`toHaveLength`, and `testEach` are pure `std/test`
additions (no compiler work) that remove most of the friction above. Module-substitution
mocking (1b) is the larger, higher-value follow-up that needs `lin-compile` support.
