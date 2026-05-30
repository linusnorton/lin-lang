# Lin benchmarks

End-to-end runtime benchmarks for **compiled Lin programs**. Each `*.lin` here is
built with the current `lin` binary and the resulting native executable is timed.
This measures the quality of the emitted code (codegen, RC, runtime), not the
speed of the Rust compiler — for that, use `cargo bench` (none wired up yet).

## Running

```bash
benchmarks/run.sh                 # build lin (release), run every benchmark
benchmarks/run.sh recursion       # only benchmarks whose name matches "recursion"
RUNS=10 benchmarks/run.sh         # more samples (default 5)
LABEL=baseline benchmarks/run.sh  # name the results file (default: git short sha)
NO_OPT=1 benchmarks/run.sh        # compile benchmarks with LIN_NO_OPT=1
```

Each benchmark is run once un-timed to warm caches, then `RUNS` timed runs; we
report the **min** (most reproducible for CPU-bound work) and **median** wall-clock
in milliseconds. Results land in `benchmarks/results/<LABEL>.txt`.

> Note: `run.sh` truncates the results file for the label it's given on each run,
> and a name filter only writes the matching rows. For a complete baseline run it
> with no filter.

## Comparing before/after a change

```bash
LABEL=before benchmarks/run.sh        # on master / before your change
# ... make the codegen/runtime change, rebuild ...
LABEL=after  benchmarks/run.sh
diff benchmarks/results/before.txt benchmarks/results/after.txt
```

Because absolute numbers depend on the machine, only compare runs taken on the
same hardware in the same session. Commit a results file only as a dated
reference point, not as a pass/fail gate.

## What each benchmark targets

| File | Hot path exercised |
|------|--------------------|
| `recursion.lin` | Function call/return overhead, TCO trampoline (tail `sumTo`), non-tail self-recursion (`fib`). Mostly unboxed Int32, so isolates call + branch cost. |
| `array_pipeline.lin` | `map`/`filter`/`reduce` over a large range: indirect closure calls, Int32 box/unbox through the `Json` element slot, RC on intermediate arrays, allocation. |
| `object_access.lin` | Object construction + the O(n) linear-scan field lookup in `lin_object`; chained field reads multiply the scan. |
| `string_build.lin` | String allocation with no small-string optimisation, interpolation/concat, string RC. |

These map to the performance findings in the project's perf review (RC traffic,
boxing at polymorphic boundaries, O(n) object field access, no SSO). When adding
an optimisation, add or extend a benchmark that targets it.

## Caveats

- Timing is whole-process wall-clock (includes process startup + module init),
  so workloads are sized to run long enough (~80 ms+) that fixed overhead is a
  small fraction. Keep new benchmarks in that range.
- `run.sh` uses bash `EPOCHREALTIME`; it needs bash ≥ 5.0.
- This is a coarse macro-harness for guiding optimisation work, not a precise
  microbenchmark framework.
