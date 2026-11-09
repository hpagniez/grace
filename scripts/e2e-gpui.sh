#!/bin/sh
# Builds GPUI Grace with its e2e hooks and runs crates/grace-e2e (migration guide R6.3). On Linux without a desktop
# it starts what AT-SPI needs: Xvfb, a session bus and the AT-SPI bus launcher. Extra arguments go to cargo test,
# e.g. a test name.
set -eu
cd "$(dirname "$0")/.."
cargo build -p grace --features e2e
if [ "$(uname)" = Linux ] && [ -z "${GRACE_E2E_DESKTOP:-}" ]; then
  export GRACE_E2E_DESKTOP=1
  : "${DISPLAY:=:99}"
  export DISPLAY
  Xvfb "$DISPLAY" -screen 0 1920x1080x24 >/dev/null 2>&1 &
  xvfb=$!
  trap 'kill $xvfb 2>/dev/null' EXIT
  exec dbus-run-session -- sh -c '/usr/libexec/at-spi-bus-launcher --launch-immediately & sleep 1; exec "$0" "$@"' "$0" "$@"
fi
cargo test -p grace-e2e --locked -- --ignored --test-threads=1 "$@"
