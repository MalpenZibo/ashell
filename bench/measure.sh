#!/usr/bin/env bash
# Measure RSS, peak RSS, wakeups/s and CPU% of a bar process.
#
#   bench/measure.sh <label> <settle_s> <sample_s> -- <command...>
#
# Starts the command, waits <settle_s> for services to settle, then samples
# /proc for <sample_s> and prints one summary line. Kills the process at the
# end. Wakeups/s = voluntary context switches per second (proxy for timer
# wakeups — what matters for battery). CPU% is over the sample window.
set -u

label=$1
settle=$2
sample=$3
shift 3
[ "$1" = "--" ] && shift

"$@" >/dev/null 2>&1 &
pid=$!
trap 'kill "$pid" 2>/dev/null' EXIT

sleep "$settle"
if ! kill -0 "$pid" 2>/dev/null; then
    echo "$label: process exited during settle" >&2
    exit 1
fi

v1=$(awk '/^voluntary_ctxt/ {print $2}' "/proc/$pid/status")
t1=$(awk '{print $14+$15}' "/proc/$pid/stat")
sleep "$sample"
if ! kill -0 "$pid" 2>/dev/null; then
    echo "$label: process exited during sample" >&2
    exit 1
fi
v2=$(awk '/^voluntary_ctxt/ {print $2}' "/proc/$pid/status")
t2=$(awk '{print $14+$15}' "/proc/$pid/stat")
rss=$(ps -o rss= -p "$pid")
hwm=$(awk '/^VmHWM/ {print $2}' "/proc/$pid/status")
clk=$(getconf CLK_TCK)

awk -v label="$label" -v rss="$rss" -v hwm="$hwm" \
    -v dv="$((v2 - v1))" -v dt="$((t2 - t1))" -v s="$sample" -v clk="$clk" \
    'BEGIN { printf "%-14s RSS %6.1f MB (peak %6.1f)  %6.2f wakeup/s  %5.2f%% CPU  (%ds sample)\n",
             label, rss/1024, hwm/1024, dv/s, dt/clk/s*100, s }'
