"""What accessibility costs the prototype during Run all on the bench's 2,000-test project (#269).

Two modes, alternated: "off" runs the app on a session bus with no AT-SPI bus, so AccessKit never starts; "on" runs it
with the AT-SPI bus and a client listening to every object event, as a screen reader does. Each round: start, wait for
the tree, 10 s idle, then Run all through stdin (the measure script's own hook) and sample the app until it finishes.

Usage: python3.12 cost.py <grace-gpui exe> <karate.jar> <project> <rounds>
"""
import os
import subprocess
import sys
import threading
import time

LISTENER = r"""
import gi, sys
gi.require_version('Atspi','2.0')
from gi.repository import Atspi
Atspi.init()
n=[0]
def cb(e): n[0]+=1
l=Atspi.EventListener.new(cb)
for t in ['object:','window:','focus:']: l.register(t)
from gi.repository import GLib
GLib.unix_signal_add(GLib.PRIORITY_DEFAULT, 15, lambda: (print(n[0], flush=True), Atspi.event_quit(), False)[-1])
Atspi.event_main()
"""


def uss(pid):
    kb = 0
    with open(f"/proc/{pid}/smaps_rollup") as f:
        for line in f:
            if line.startswith(("Private_Clean", "Private_Dirty")):
                kb += int(line.split()[1])
    return kb / 1024


def cpu_ms(pid):
    with open(f"/proc/{pid}/stat") as f:
        parts = f.read().rsplit(")", 1)[1].split()
    return (int(parts[11]) + int(parts[12])) * 1000 / os.sysconf("SC_CLK_TCK")


def bus(with_atspi):
    out = subprocess.run(
        ["dbus-daemon", "--session", "--fork", "--print-address=1", "--print-pid=1"], capture_output=True, text=True
    ).stdout.split()
    addr, pid = out[0], int(out[1])
    env = dict(os.environ, DBUS_SESSION_BUS_ADDRESS=addr)
    procs = [pid]
    if with_atspi:
        launcher = subprocess.Popen(["/usr/libexec/at-spi-bus-launcher", "--launch-immediately"], env=env,
                                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(1.5)
        procs.append(launcher)
    return env, procs


def round_(exe, jar, project, mode):
    env, procs = bus(mode == "on")
    env = {k: v for k, v in env.items() if k != "JAVA_TOOL_OPTIONS"}
    env.update(GRACE_PROBE="1", GRACE_JAVA=env.get("GRACE_JAVA", "java"), GRACE_KARATE_JAR=jar)
    listener = None
    if mode == "on":
        listener = subprocess.Popen(["/usr/bin/python3.12", "-c", LISTENER], env=env, stdout=subprocess.PIPE,
                                    stderr=subprocess.DEVNULL, text=True)
        time.sleep(1)
    app = subprocess.Popen([exe, project], env=env, stdin=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    probes = {}

    def read():
        for line in app.stderr:
            if line.startswith("PROBE "):
                probes[line.split()[1]] = time.time()

    threading.Thread(target=read, daemon=True).start()
    while "tree_shown" not in probes:
        time.sleep(0.05)
    time.sleep(10)
    idle_mem = uss(app.pid)
    c0 = cpu_ms(app.pid)
    time.sleep(10)
    idle_cpu = cpu_ms(app.pid) - c0
    c0 = cpu_ms(app.pid)
    t0 = time.time()
    app.stdin.write("run\n")
    app.stdin.flush()
    peak = 0
    while "run_finished" not in probes:
        peak = max(peak, uss(app.pid))
        time.sleep(0.25)
    run_cpu = cpu_ms(app.pid) - c0
    wall = probes["run_finished"] - t0
    after = uss(app.pid)
    app.terminate()
    app.wait()
    events = None
    if listener:
        listener.terminate()
        events = listener.communicate(timeout=5)[0].strip()
    for p in procs:
        if isinstance(p, int):
            os.kill(p, 15)
        else:
            p.terminate()
    return dict(mode=mode, idle_mb=round(idle_mem, 1), idle_cpu_ms_10s=round(idle_cpu), peak_mb=round(peak, 1),
                after_mb=round(after, 1), run_cpu_ms=round(run_cpu), run_s=round(wall, 2), atspi_events=events)


def main(exe, jar, project, rounds):
    round_(exe, jar, project, "off")  # warm-up
    for r in range(int(rounds)):
        for mode in ("off", "on") if r % 2 == 0 else ("on", "off"):
            print(round_(exe, jar, project, mode), flush=True)


if __name__ == "__main__":
    main(*sys.argv[1:5])
