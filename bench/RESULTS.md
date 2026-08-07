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

## Nota (2026-08-07, sera)

I numeri CPU sopra vanno rivisti: le misure erano inquinate da burst
intermittenti del main thread (fino a ~30% per secondi) causati da un bug
core di guido, non dal port: la coda job è globale ma il frame-pacing gate
è per superficie — la superficie menu (idle, gate aperto) drena i job di
animazione della bar senza pacing (~260k iterazioni/s durante le spring
`animate_width` dei pill workspace). Confermato rimuovendo la superficie
menu: 87 → 4 tick/3s. RSS e wakeup/s restano validi. CPU da rimisurare
dopo il fix in guido (pacing per superficie dei job di animazione).
Il cap tokio worker_threads=2 è stato rimosso: A/B interleaved non mostra
alcun effetto (il delta era il rumore dei burst di cui sopra).

## 2026-08-07 — dopo il fix guido #118 (pacing per-superficie)

Bug confermato e corretto in guido (PR #118, mergiata): worst-case del
repro workspaces+menu da 87 a 2 tick/3s. Nuovo A/B 60s:

| bar | RSS | wakeups/s | CPU |
|---|---|---|---|
| ashell-iced 0.9.0 | 181.9 MB | 5.97 | 0.15% |
| ashell-guido      | 132.4 MB | 0.60 | 0.58% |

RSS −27%, wakeup −90% (stabili run dopo run). La CPU residua resta
sopra iced e varia tra i campioni (~0.3–0.6%): è lato servizi/da
caratterizzare meglio (cadenza sysinfo, costo animazioni legittime a
60fps), non più il busy-spin. Prossimo focus CPU quando rientra nel
lavoro sui moduli.
