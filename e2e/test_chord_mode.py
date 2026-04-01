"""Chord mode: structure selection shows hint + mute, hides pent. Tapping a fret builds a voicing."""

import time
from dogtail.predicate import GenericPredicate
from e2e.helpers import TestContext, dbus_action


def test_chord_mode(ctx: TestContext):
    print("\n== Chord Mode ==")
    ctx.switch_to_scales()
    time.sleep(0.5)

    # Select Triad via D-Bus
    ok = dbus_action("set-chord-structure", "[<uint32 1>]")
    ctx.check(ok, "D-Bus set-chord-structure(Triad) succeeded")
    time.sleep(0.5)
    ctx.screenshot("chord_triad_selected")

    # Tap hint visible
    hint = ctx.find(lambda n: "Tap a note" in (n.name or ""))
    ctx.check(hint is not None and hint.showing,
              "Tap hint visible when chord structure set, no note tapped")

    # Mute toggle visible
    mute = ctx.find(lambda n: n.roleName == "toggle button" and n.showing)
    ctx.check(mute is not None, f"Mute toggle visible (name: '{mute.name if mute else None}')")

    # Pent hidden in chord mode
    pent = ctx.find(GenericPredicate(roleName="push button", name="Pent"))
    pent_hidden = pent is None or not pent.showing
    ctx.check(pent_hidden, "Pent button hidden in chord mode")

    # Tap fret via D-Bus (string 2, fret 3 = F in C major)
    ok = dbus_action("tap-fret", "[<(uint32 2, uint32 3)>]")
    ctx.check(ok, "D-Bus tap-fret(2, 3) succeeded")
    time.sleep(0.5)
    ctx.screenshot("chord_voicing_built")

    # Hint should be gone now
    hint = ctx.find(lambda n: "Tap a note" in (n.name or "") and n.showing)
    ctx.check(hint is None, "Tap hint hidden after degree selected")

    # Reset
    dbus_action("set-chord-structure", "[<uint32 0>]")
    time.sleep(0.3)

    # Pent should reappear
    pent = ctx.find(GenericPredicate(roleName="push button", name="Pent"))
    ctx.check(pent is not None and pent.showing,
              "Pent reappears after exiting chord mode")


if __name__ == "__main__":
    from e2e.helpers import clean_screenshots
    clean_screenshots()
    ctx = TestContext()
    test_chord_mode(ctx)
    ctx.summarize()
