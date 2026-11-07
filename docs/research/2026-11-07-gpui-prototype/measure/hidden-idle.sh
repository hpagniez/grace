#!/bin/bash
# Idle CPU and thread wake-ups of an app's whole process tree, shown and then hidden in three ways, for ADR 0030
# (no wake-ups while minimised or hidden; a budget while shown). Linux only: X11 with a window manager (openbox here).
#
#   hidden-idle.sh electron <Grace exe> <project>
#   hidden-idle.sh gpui <grace-gpui exe> <project> <java> <karate.jar>
#
# Each state: 10 s to settle, then 60 s measured. States, in order:
#   shown      the window mapped and on top
#   minimised  iconified through the window manager (xdotool windowminimize)
#   covered    shown, but fully covered by a maximised xterm
#   workspace  moved to another workspace (the window manager unmaps it)
# CPU is utime + stime of every process in the tree (/proc/<pid>/stat, CLK_TCK 100, so 10 ms resolution);
# wake-ups are the voluntary context switches of all their threads (/proc/<pid>/task/*/status).
set -eu
export DISPLAY=${DISPLAY:-:99}
SETTLE=${SETTLE:-10}
SPAN=${SPAN:-60}

tree() { # the pid and all its descendants
  local p=$1; echo "$p"
  for c in $(cat /proc/"$p"/task/*/children 2>/dev/null); do tree "$c"; done
}
snap() { # "pid cpu_ms wakeups" for each process in the tree
  for p in $(tree "$1"); do
    echo "$p $(awk '{print ($14+$15)*10}' /proc/"$p"/stat 2>/dev/null || echo 0) $(cat /proc/"$p"/task/*/status 2>/dev/null | awk '/^voluntary_ctxt/{s+=$2} END{print s+0}')"
  done
}
# A process that starts during the span counts from zero; one that ends during it is left out.
window() {
  local a b; a=$(mktemp); b=$(mktemp)
  sleep "$SETTLE"; snap "$1" > "$a"; sleep "$SPAN"; snap "$1" > "$b"
  awk 'NR==FNR{c[$1]=$2; w[$1]=$3; next} {cpu+=$2-c[$1]; wk+=$3-w[$1]} END{printf "cpu_ms=%d wakeups=%d procs=%d", cpu, wk, FNR}' "$a" "$b"
  rm -f "$a" "$b"
}

DATA=$(mktemp -d)
case $1 in
  electron)
    printf '{"lastProject":"%s","updates":{"checkAtStartup":false},"window":{"x":0,"y":0,"width":1400,"height":900,"maximized":false}}' "$3" > "$DATA/settings.json"
    GRACE_USER_DATA=$DATA "$2" --no-sandbox >/dev/null 2>&1 & pid=$!
    sleep 15 ;;
  gpui)
    mkfifo "$DATA/in"; sleep 100000 > "$DATA/in" & holder=$!
    GRACE_PROBE=1 GRACE_USER_DATA=$DATA GRACE_JAVA=$4 GRACE_KARATE_JAR=$5 "$2" "$3" < "$DATA/in" 2> "$DATA/err.log" & pid=$!
    until grep -q "PROBE tree_shown" "$DATA/err.log"; do sleep 0.1; done ;;
esac
win=""
until [ -n "$win" ]; do win=$(xdotool search --onlyvisible --pid "$pid" 2>/dev/null | tail -1 || true); sleep 0.2; done

echo "$1 shown $(window "$pid")"
xdotool windowminimize "$win"
[ -n "${SHOTS:-}" ] && import -window root "$SHOTS/$1-minimised.png"
echo "$1 minimised $(window "$pid")"
wmctrl -i -a "$win"
xterm -geometry 400x200+0+0 & cover=$!
sleep 1; wmctrl -r :ACTIVE: -b add,maximized_vert,maximized_horz
echo "$1 covered $(window "$pid")"
[ -n "${SHOTS:-}" ] && import -window root "$SHOTS/$1-covered.png"
kill $cover
wmctrl -i -a "$win"; sleep 1
wmctrl -i -r "$win" -t 1
echo "$1 workspace $(window "$pid")"
wmctrl -s 0
kill "$pid" ${holder:-} 2>/dev/null || true
pkill -P "$pid" 2>/dev/null || true
rm -rf "$DATA"
