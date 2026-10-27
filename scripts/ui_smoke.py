#!/usr/bin/env python3
"""Exercise tray lifecycle and render offline UI fixtures on native Wayland.

Requires Weston, dbus-run-session and PyGObject (python3-gi on Ubuntu).
The compositor, session bus and application data are private to this run.
"""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time


class TrayHost:
    """Minimal real StatusNotifierWatcher for an isolated integration test."""

    NAME = "org.kde.StatusNotifierWatcher"
    PATH = "/StatusNotifierWatcher"
    XML = """<node><interface name="org.kde.StatusNotifierWatcher">
      <method name="RegisterStatusNotifierItem"><arg type="s" direction="in"/></method>
      <method name="RegisterStatusNotifierHost"><arg type="s" direction="in"/></method>
      <property name="RegisteredStatusNotifierItems" type="as" access="read"/>
      <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
      <property name="ProtocolVersion" type="i" access="read"/>
      <signal name="StatusNotifierItemRegistered"><arg type="s"/></signal>
      <signal name="StatusNotifierItemUnregistered"><arg type="s"/></signal>
      <signal name="StatusNotifierHostRegistered"/>
      <signal name="StatusNotifierHostUnregistered"/>
    </interface></node>"""

    def __init__(self, control):
        from gi.repository import Gio, GLib

        self.GLib, self.Gio = GLib, Gio
        self.bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        self.service = None
        self.path = "/StatusNotifierItem"
        self.control = control
        self.last_command = ""
        self.registrations = 0
        info = Gio.DBusNodeInfo.new_for_xml(self.XML)
        self.registration = self.bus.register_object(
            self.PATH, info.interfaces[0], self.method, self.property, None
        )
        self.own_name()

    def call(self, service, path, interface, method, signature, values):
        return self.bus.call_sync(
            service, path, interface, method,
            self.GLib.Variant(signature, values), None,
            self.Gio.DBusCallFlags.NONE, 5000, None,
        ).unpack()

    def own_name(self):
        result = self.call(
            "org.freedesktop.DBus", "/org/freedesktop/DBus",
            "org.freedesktop.DBus", "RequestName", "(su)", (self.NAME, 0),
        )
        if result[0] != 1:
            raise RuntimeError("Could not acquire the private tray watcher name")

    def method(self, connection, sender, path, interface, method, params, invocation):
        if method == "RegisterStatusNotifierItem":
            service = params.unpack()[0]
            self.service = sender if service.startswith("/") else service
            self.path = service if service.startswith("/") else "/StatusNotifierItem"
            self.registrations += 1
        invocation.return_value(self.GLib.Variant("()", ()))

    def property(self, connection, sender, path, interface, name):
        if name == "RegisteredStatusNotifierItems":
            items = [self.service + self.path] if self.service else []
            return self.GLib.Variant("as", items)
        if name == "IsStatusNotifierHostRegistered":
            return self.GLib.Variant("b", True)
        if name == "ProtocolVersion":
            return self.GLib.Variant("i", 0)
        return None

    def tick(self):
        context = self.GLib.MainContext.default()
        while context.pending():
            context.iteration(False)
        command_file = self.control / "command"
        command = command_file.read_text() if command_file.exists() else ""
        if not command or command == self.last_command:
            return
        self.last_command = command
        if command == "activate":
            if not self.service:
                raise RuntimeError("Tray was never registered")
            properties = self.call(
                self.service, self.path, "org.freedesktop.DBus.Properties",
                "GetAll", "(s)", ("org.kde.StatusNotifierItem",),
            )[0]
            if properties.get("Id") != "sing-box-gui":
                raise RuntimeError("Wrong tray application ID")
            icons = properties.get("IconPixmap", [])
            if not icons or any(len(data) != w * h * 4 for w, h, data in icons):
                raise RuntimeError("Invalid tray icon pixmaps")
            _, layout = self.call(
                self.service, properties["Menu"], "com.canonical.dbusmenu",
                "GetLayout", "(iias)", (0, -1, []),
            )
            if len(layout[2]) < 5:
                raise RuntimeError("Tray menu is incomplete")
            self.call(
                self.service, self.path, "org.kde.StatusNotifierItem",
                "Activate", "(ii)", (0, 0),
            )
        elif command == "remove-watcher":
            self.call(
                "org.freedesktop.DBus", "/org/freedesktop/DBus",
                "org.freedesktop.DBus", "ReleaseName", "(s)", (self.NAME,),
            )
        elif command == "restore-watcher":
            self.own_name()
        else:
            raise RuntimeError(f"Unknown tray test command: {command}")


def stop(process):
    if process is None or process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def run_session(args):
    started = time.monotonic()
    runtime = Path(os.environ["XDG_RUNTIME_DIR"])
    output = args.output.resolve()
    env = os.environ.copy()
    weston = "weston"
    client = ""
    if args.weston_root:
        root = args.weston_root.resolve() / "usr"
        weston = str(root / "bin/weston")
        modules = root / "lib64"
        env["LD_LIBRARY_PATH"] = f"{modules}:{modules}/weston"
        backend = next(modules.glob("libweston-*/headless-backend.so"))
        env["WESTON_MODULE_MAP"] = (
            f"headless-backend.so={backend};"
            f"desktop-shell.so={modules}/weston/desktop-shell.so"
        )
        client = f"client={root}/libexec/weston-desktop-shell\n"
    version = subprocess.check_output([weston, "--version"], env=env, text=True)
    major = int(re.search(r"\b(\d+)\.", version)[1])
    config = runtime / "weston.ini"
    config.write_text(
        f"[core]\nidle-time=0\n[shell]\n{client}panel-position=none\nlocking=false\n"
        "background-type=solid\nbackground-color=0xff202020\nanimation=none\n"
        "startup-animation=none\n[input-method]\npath=\n"
    )
    (output / "weston.log").write_text("")
    command = [
        weston, "--backend=headless-backend.so", "--shell=desktop-shell.so",
        f"--width={1920 * args.scale}", f"--height={1200 * args.scale}",
        f"--scale={args.scale}", "--socket=wayland-smoke",
        f"--config={config}", f"--log={output / 'weston.log'}",
    ]
    # Iced renders and captures its own surface. A headless compositor only
    # needs to drive the native Wayland lifecycle/scale/frame callbacks; noop
    # also avoids driver-specific output compositing bugs in CI.
    if major >= 12:
        command.append("--renderer=noop")
    compositor = application = None
    with (output / "compositor.log").open("w") as compositor_log, \
            (output / "application.log").open("w") as app_log:
        try:
            compositor = subprocess.Popen(
                command, env=env, stdout=compositor_log, stderr=subprocess.STDOUT
            )
            deadline = time.monotonic() + 15
            while not (runtime / "wayland-smoke").exists():
                if compositor.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError("Weston failed to create a native Wayland socket")
                time.sleep(0.05)
            host = None if args.no_tray else TrayHost(Path(env["SINGBOX_UI_SMOKE_CONTROL_DIR"]))
            application = subprocess.Popen(
                [str(args.binary.resolve())], env=env, stdout=app_log, stderr=subprocess.STDOUT,
            )
            deadline = time.monotonic() + 600
            while application.poll() is None:
                if host:
                    host.tick()
                if compositor.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError("Wayland UI smoke check timed out or compositor exited")
                time.sleep(0.01)
            if application.returncode:
                raise RuntimeError(f"Application exited with code {application.returncode}")
            if host and host.registrations < 2:
                raise RuntimeError("Tray did not register again after watcher recovery")
        finally:
            stop(application)
            stop(compositor)
    log = (output / "application.log").read_text()
    match = re.search(r"UI_SMOKE_OK: (\d+) screenshots", log)
    tray_marker = "UI_TRAY_NONE_OK:" if args.no_tray else "UI_TRAY_OK:"
    if not match or int(match[1]) == 0 or tray_marker not in log:
        raise RuntimeError("Screenshot or tray validation did not finish")
    if len(list(output.glob("*.png"))) < int(match[1]):
        raise RuntimeError("Some screenshots are missing")
    result = {
        "renderer": args.backend,
        "scale": args.scale,
        "screenshots": int(match[1]),
        "tray_host": not args.no_tray,
        "tray_checks": "passed",
        "weston": version.strip(),
        "elapsed_seconds": round(time.monotonic() - started, 2),
    }
    (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"Wayland {args.scale}x ({args.backend}): {match[1]} screenshots; tray checks passed; output: {output}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path("target/ui-smoke"))
    parser.add_argument("--binary", type=Path, default=Path("target/debug/sing-box-gui"))
    parser.add_argument("--weston-root", type=Path, help="Optional unpacked Weston prefix")
    parser.add_argument("--filter", default="")
    parser.add_argument("--scale", type=int, choices=[1, 2], default=1)
    parser.add_argument("--backend", choices=["wgpu", "tiny-skia"], default="wgpu")
    parser.add_argument("--no-tray", action="store_true", help="Test a desktop without a tray host")
    parser.add_argument("--session", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.session:
        run_session(args)
        return
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="sing-box-wayland-") as temp:
        runtime = Path(temp)
        control = runtime / "control"
        control.mkdir()
        env = os.environ.copy()
        for name in ("DISPLAY", "WAYLAND_SOCKET"):
            env.pop(name, None)
        env.update(
            XDG_RUNTIME_DIR=str(runtime), XDG_SESSION_TYPE="wayland",
            XDG_CONFIG_HOME=str(runtime / "config"), XDG_CACHE_HOME=str(runtime / "cache"),
            XDG_DATA_HOME=str(runtime / "share"), XDG_STATE_HOME=str(runtime / "state"),
            WAYLAND_DISPLAY="wayland-smoke", ICED_BACKEND=args.backend,
            SING_BOX_GUI_DATA_DIR=str(runtime / "data"),
            SINGBOX_UI_SMOKE_DIR=str(output), SINGBOX_UI_SMOKE_FILTER=args.filter,
            SINGBOX_UI_SMOKE_SCALE=str(args.scale),
            SINGBOX_UI_SMOKE_TRAY="0" if args.no_tray else "1",
            SINGBOX_UI_SMOKE_CONTROL_DIR=str(control),
        )
        result = subprocess.run(
            ["dbus-run-session", "--", sys.executable, str(Path(__file__).resolve()),
             *sys.argv[1:], "--session"],
            env=env, capture_output=True, text=True, timeout=640,
        )
        print(result.stdout, end="")
        if result.returncode:
            print(result.stderr)
            for name in ("application.log", "weston.log"):
                path = output / name
                if path.exists():
                    print(path.read_text()[-8000:])
            raise SystemExit("Wayland UI smoke check failed; see the output logs")


if __name__ == "__main__":
    main()
