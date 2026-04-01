"""Shared utilities for E2E tests.

Interaction strategy:
- Buttons/tabs: dogtail do_action(0) via AT-SPI
- Combo boxes / fretboard: D-Bus GAction activation
- Screenshots: import(1) by window ID + ImageMagick resize
"""

import os
import subprocess
import sys

from dogtail.tree import root
from dogtail.predicate import GenericPredicate

SCREENSHOT_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), "e2e_screenshots")
SCREENSHOT_SCALE = "30%"
DBUS_DEST = "com.adrienplazas.Metronome"
DBUS_PATH = "/com/adrienplazas/Metronome"


class TestContext:
    """Shared state for a test run."""

    def __init__(self):
        self.failures = []
        self.app = root.application("metronome")

    def screenshot(self, name: str) -> str:
        os.makedirs(SCREENSHOT_DIR, exist_ok=True)
        raw = os.path.join(SCREENSHOT_DIR, f"{name}_raw.png")
        final = os.path.join(SCREENSHOT_DIR, f"{name}.png")
        wid = subprocess.check_output(
            ["xdotool", "search", "--name", "Metronome"], text=True
        ).strip().split("\n")[-1]
        subprocess.run(["import", "-window", wid, raw], check=True)
        subprocess.run(
            ["convert", raw, "-resize", SCREENSHOT_SCALE, final], check=True
        )
        os.remove(raw)
        print(f"  -> {name}.png")
        return final

    def check(self, condition: bool, message: str):
        if not condition:
            self.failures.append(message)
            print(f"  FAIL: {message}")
        else:
            print(f"  OK: {message}")

    def find(self, predicate):
        """Find a child, returning None if not found."""
        try:
            return self.app.findChild(predicate, retry=False)
        except Exception:
            return None

    def click(self, node):
        """Click via AT-SPI action (no coordinates)."""
        node.do_action(0)

    def switch_to_scales(self):
        tab = self.app.findChild(GenericPredicate(roleName="page tab", name="Scales"))
        self.click(tab)

    def switch_to_metronome(self):
        tab = self.app.findChild(GenericPredicate(roleName="page tab", name="Metronome"))
        self.click(tab)

    def summarize(self):
        print(f"\n{'='*40}")
        if self.failures:
            print(f"FAILED — {len(self.failures)} check(s):")
            for f in self.failures:
                print(f"  - {f}")
            sys.exit(1)
        else:
            print("ALL CHECKS PASSED")


def dbus_action(action_name: str, param_str: str = "[]") -> bool:
    """Invoke a GAction on the app via D-Bus."""
    result = subprocess.run(
        [
            "gdbus", "call", "--session",
            "--dest", DBUS_DEST,
            "--object-path", DBUS_PATH,
            "--method", "org.freedesktop.Application.ActivateAction",
            action_name, param_str, "{}",
        ],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"  DBUS ERROR: {result.stderr.strip()}")
    return result.returncode == 0


def clean_screenshots():
    script = os.path.join(os.path.dirname(os.path.dirname(__file__)), "e2e_clean.sh")
    subprocess.run([script], check=True)
