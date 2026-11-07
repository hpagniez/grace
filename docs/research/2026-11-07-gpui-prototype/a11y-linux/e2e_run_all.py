"""e2e/run.spec.ts "Run all with live progress", ported to the accessibility tree (AT-SPI) (#269).

It drives the prototype only through what a screen reader sees: it finds nodes by role and name, presses them with
their AT-SPI action, and reads names, descriptions, values and announcements. No pixels, no coordinates.

Usage: python3.12 e2e_run_all.py <grace-gpui exe> <karate.jar> <copy of test/fixtures/projects/clean>
Needs DISPLAY, DBUS_SESSION_BUS_ADDRESS and a running at-spi-bus-launcher (README.md).
"""
import os
import re
import subprocess
import sys
import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, GLib  # noqa: E402

Atspi.init()
announcements = []


def on_announcement(event):
    announcements.append(event.any_data if isinstance(event.any_data, str) else event.source.get_name())


listener = Atspi.EventListener.new(on_announcement)
listener.register("object:announcement")


def pump():
    ctx = GLib.MainContext.default()
    while ctx.iteration(False):
        pass


def app_root(name):
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        app = desktop.get_child_at_index(i)
        if app is not None and app.get_name() == name:
            return app
    return None


def nodes(root):
    out, stack = [], [root]
    while stack:
        a = stack.pop()
        try:
            out.append(a)
            stack.extend(c for c in (a.get_child_at_index(i) for i in range(a.get_child_count())) if c is not None)
        except GLib.Error:
            pass  # a row that scrolled away while we walked
    return out


def find(root, role, name=None, desc=None):
    for a in nodes(root):
        try:
            if a.get_role_name() == role and (name is None or a.get_name() == name) and (
                desc is None or a.get_description() == desc
            ):
                return a
        except GLib.Error:
            pass
    return None


def wait(what, check, timeout):
    end = time.time() + timeout
    while time.time() < end:
        pump()
        got = check()
        if got:
            return got
        time.sleep(0.05)
    raise AssertionError(f"timed out after {timeout} s waiting for {what}")


def press(node):
    assert node.get_action_iface().do_action(0), "AT-SPI action refused"


def child(node, role, name):
    for i in range(node.get_child_count()):
        c = node.get_child_at_index(i)
        if c is not None and c.get_role_name() == role and c.get_name() == name:
            return c
    return None


def main(exe, jar, project):
    env = {k: v for k, v in os.environ.items() if k != "JAVA_TOOL_OPTIONS"}
    env.update(GRACE_JAVA=env.get("GRACE_JAVA", "java"), GRACE_KARATE_JAR=jar)
    started = time.time()
    proc = subprocess.Popen([exe, project], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        app = wait("the app in the AT-SPI tree", lambda: app_root("grace-gpui"), 15)
        # libatspi caches each node's interfaces and states; the test must read them live.
        app.set_cache_mask(Atspi.Cache.NONE)
        # openClean(): the project is open and its features are listed.
        basic = wait("basic.feature", lambda: find(app, "tree item", "basic.feature"), 15)
        for feature in ("basic.feature", "slow.feature"):
            row = find(app, "tree item", feature)
            press(child(row, "push button", "Open"))
            wait(f"{feature} open", lambda: child(find(app, "tree item", feature), "push button", "Close"), 5)

        # A disabled Kit button still says "enabled" on AT-SPI; it only loses its click action, so wait for that.
        run_all = wait("Run all pressable", lambda: (b := find(app, "push button", "Run all")) and b.get_action_iface() and b, 15)
        press(run_all)
        # Two status nodes: the summary ("0 of 9 · 0.1 s") and the announcer ("Running 9 tests").
        status = lambda: next(  # noqa: E731
            (n for a in nodes(app) if a.get_role_name() == "status bar" and re.match(r"\d+ of \d+", n := a.get_name())),
            "",
        )
        wait('"0 of 9"', lambda: status().startswith("0 of 9"), 15)
        wait('"slow loop" running', lambda: find(app, "tree item", "slow loop", "Running"), 45)
        assert status().split(" · ")[0] != "9 of 9", "still running"
        wait("a scenario passed", lambda: find(app, "tree item", None, "Passed"), 45)
        wait("a scenario failed", lambda: find(app, "tree item", None, "Failed"), 45)

        wait('"9 of 9"', lambda: status().startswith("9 of 9"), 45)
        bar = find(app, "progress bar", "Run progress").get_value_iface()
        assert bar.get_current_value() == 9 and bar.get_maximum_value() == 9, (
            bar.get_current_value(),
            bar.get_maximum_value(),
        )
        wait("the end of the run announced", lambda: any(a.startswith("Run ") and "passed" in a for a in announcements), 5)
        end = [a for a in announcements if a.startswith("Run ")][-1]
        assert end == "Run failed: 8 passed, 1 failed", end
        assert find(app, "tree item", "basic.feature").get_description() == "Failed"
        assert announcements[0] == "Running 9 tests", announcements
        print(f"PASS in {time.time() - started:.1f} s; announced: {announcements}")
    finally:
        proc.terminate()
        proc.wait()


if __name__ == "__main__":
    main(*sys.argv[1:4])
