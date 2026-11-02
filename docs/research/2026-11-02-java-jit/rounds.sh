#!/usr/bin/env bash
# Spike for #179: interleaved rounds of the JIT candidates, on top of the heap flags Grace passes today.
#   bash rounds.sh <repo> <rounds> <out.jsonl> <case>...   (case: 2k:1, 10k:1, 10k:8, 10k:dry, js:1, js:8)
set -u
repo=$1 rounds=$2 out=$3
shift 3
measure="$(dirname "$0")/measure.mjs"
heap="-Xms64m -Xmx2g -XX:+ExitOnOutOfMemoryError"
declare -A flags=(
  [today]=""
  [c1]="-XX:TieredStopAtLevel=1"
  [cc2]="-XX:CICompilerCount=2"
)
for r in $(seq "$rounds"); do
  for c in "$@"; do
    size=${c%%:*} t=${c##*:} dry=""
    [ "$t" = dry ] && t=1 dry=--dry
    for k in today c1 cc2; do
      # shellcheck disable=SC2086
      node --no-warnings "$measure" "$repo" "$size" "$t" "$k" $dry -- $heap ${flags[$k]} | tee -a "$out"
    done
  done
done
