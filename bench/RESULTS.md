# Benchmark results

Method: `bench/measure.sh`, identical module set (Workspaces / WindowTitle /
SystemInfo+Clock|Tempo+Settings), 15s settle + 60s idle sample, niri, quiet
system. ashell-iced is the installed 0.9.0 (= upstream main 23a5d136).

## 2026-08-07 — after the first perf pass on the port

| bar | RSS | wakeups/s | CPU |
|---|---|---|---|
| ashell-iced 0.9.0 | 182.2 MB | 6.03 | 0.17% |
| ashell-guido      | 130.1 MB | **0.57** | 0.33% |

- RSS **−29%**; wakeups **−90%** (the battery driver: each wakeup exits
  package C-states).
- guido's main (render) thread uses ~10–20 ms CPU per 30 s at idle — the
  residual CPU is service-side (sysinfo sampling cadence).
- Port perf pass: sysinfo refresh scoped to configured indicators (single
  temperature sensor instead of every hwmon node), full refresh only while
  the menu is open, minute-aligned clock wakeups, tokio capped at 2 workers.

## Baseline before the pass (30s samples)

| bar | RSS | wakeups/s | CPU |
|---|---|---|---|
| ashell-iced 0.9.0 | 182.2 MB | 5.90 | 0.13% |
| ashell-guido      | 131.6 MB | 1.30 | 0.57% |
