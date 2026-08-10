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

## 2026-08-07 — menu come popup xdg (guido #120)

Memoria GPU (amdgpu vram+gtt, campionamento a minimo per filtrare il
rumore di altri processi, schermo 2880×1800, config bench):

| build | GPU dell'app |
|---|---|
| overlay fullscreen (pre-popup) | +353 / +340 MiB |
| menu popup | +81 / +135 MiB |

≈250 MiB risparmiati per monitor. RSS/wakeup/CPU invariati (131.5 MB /
0.58 wk/s / ~0.5%). Nota: la verifica interattiva dei menu (click,
grab, posizionamento, contenuti) va fatta a mano.

## 2026-08-10 — servizi upstream via compat layer

A/B 20s, sistema in uso ma stabile, dopo pulizia istanze:

| bar | RSS | wakeups/s (main) | CPU |
|---|---|---|---|
| ashell-iced 0.9.0 | 183.5 MB | 5.95 | 0.25% |
| ashell-guido (servizi upstream) | 134.9 MB | **0.85** | **0.20%** |

**Il gap di CPU residua è chiuso**: con i servizi identici a upstream
(compat layer, zero polling is_running nel runner) la CPU è a pari/sotto
iced. RSS −26%, wakeup main −86%.

Errori di misura corretti strada facendo (entrambi documentati perché
costati un'ora): (1) una run A/B partita subito dopo una build — regola
nota, 15.5 wk/s fasulli; (2) un'istanza orfana della bar (kill sul PID
della subshell) ha contaminato la prima serie di bisect.

**Metrica nuova — wakeup a livello PROCESSO** (mai misurata prima, tutte
le cifre storiche sono main-thread-only per entrambe le bar):

| bar | main | totale processo |
|---|---|---|
| iced | 7.4 | 24.0 |
| guido full | 1.1 | 64.9 |
| guido, sei servizi spenti (ASHELL_BENCH_ONLY=None) | ~0.6 | 37.5 |

I worker tokio di guido si svegliano ~2.7× iced: ~37/s dalla nostra
baseline mai swappata (sysinfo a fette da 500ms, stream compositor,
ecc.) + ~27/s dai sei servizi upstream. CPU comunque a 0.20% — sono
micro-wake. Prossimo focus batteria: caratterizzare e ridurre i wakeup
worker-side (è qui che si vince ora, non più sul main).

Strumentazione aggiunta: `ASHELL_BENCH_ONLY=Frag1,Frag2` limita i
servizi compat avviati; `RUST_LOG=...compat=debug` conta i publish.
