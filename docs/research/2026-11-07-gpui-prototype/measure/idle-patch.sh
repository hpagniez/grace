#!/bin/bash
# Idle CPU and thread wake-ups of a GPUI exe, with and without the idle patches (#268). Linux only, under Xvfb.
#   idle-patch.sh floor <floor exe>          the empty window: 20 s to settle, then 60 s
#   idle-patch.sh app <grace-gpui exe> <project> <karate.jar>
#       the prototype with a project open: 20 s after the tree shows, 60 s; then Run all,
#       a click in the tree, 20 s, and 60 s again. Screenshots go to $OUT (default ./idle-out).
# Wake-ups are the voluntary context switches of all the process's threads (/proc/<pid>/task/*/status).
set -eu
export DISPLAY=${DISPLAY:-:99}
cpu() { awk '{print ($14+$15)*10}' /proc/$1/stat; }   # ms, CLK_TCK = 100
ctx() { cat /proc/$1/task/*/status | awk '/^voluntary_ctxt/{s+=$2} END{print s}'; }
window() { local t0 c0; t0=$(cpu $1); c0=$(ctx $1); sleep 60; echo "cpu_ms=$(( $(cpu $1) - t0 )) wakeups=$(( $(ctx $1) - c0 ))"; }
case $1 in
  floor)
    "$2" >/dev/null 2>&1 & pid=$!
    sleep 20; echo "floor idle $(window $pid)"; kill $pid ;;
  app)
    OUT=${OUT:-idle-out}; mkdir -p "$OUT"; rm -f "$OUT/in" "$OUT/err.log"; mkfifo "$OUT/in"
    sleep 100000 > "$OUT/in" & holder=$!
    GRACE_PROBE=1 GRACE_JAVA=${GRACE_JAVA:-java} GRACE_KARATE_JAR="$4" GRACE_USER_DATA="$OUT/data" "$2" "$3" < "$OUT/in" 2> "$OUT/err.log" & pid=$!
    probe() { until grep -q "PROBE $1" "$OUT/err.log"; do sleep 0.1; done; }
    probe tree_shown; sleep 20; echo "app idle-with-project $(window $pid)"
    echo run > "$OUT/in"; probe run_finished; sleep 1; import -window root "$OUT/after-run.png"
    xdotool mousemove 45 157 click 1; sleep 0.5; xdotool mousemove 120 181 click 1; sleep 0.5
    import -window root "$OUT/after-click.png"
    sleep 20; echo "app idle-after-run $(window $pid)"
    echo quit > "$OUT/in"; sleep 1; kill $pid $holder 2>/dev/null || true ;;
esac
