"""Mute toggle switches label between 🔊 and 🔇 and controls chord playback."""

import time
from e2e.helpers import TestContext, dbus_action


def test_mute_toggle(ctx: TestContext):
    print("\n== Mute Toggle ==")
    ctx.switch_to_scales()
    time.sleep(0.5)

    # Set up: Triad chord, tap a note so mute button is actionable
    dbus_action("set-chord-structure", "[<uint32 1>]")
    time.sleep(0.3)
    dbus_action("tap-fret", "[<(uint32 2, uint32 3)>]")
    time.sleep(0.5)

    mute = ctx.find(lambda n: n.roleName == "toggle button" and n.showing)
    ctx.check(mute is not None, "Mute toggle visible in chord mode")
    if not mute:
        return

    ctx.check("\U0001F50A" in mute.name, f"Initial label is 🔊 (got: '{mute.name}')")

    # Click to mute
    ctx.click(mute)
    time.sleep(0.3)
    ctx.screenshot("mute_on")
    mute = ctx.find(lambda n: n.roleName == "toggle button" and n.showing)
    ctx.check(mute is not None and "\U0001F507" in mute.name,
              f"After mute: label is 🔇 (got: '{mute.name if mute else None}')")

    # Click to unmute
    ctx.click(mute)
    time.sleep(0.3)
    ctx.screenshot("mute_off")
    mute = ctx.find(lambda n: n.roleName == "toggle button" and n.showing)
    ctx.check(mute is not None and "\U0001F50A" in mute.name,
              f"After unmute: label is 🔊 (got: '{mute.name if mute else None}')")

    # Cleanup
    dbus_action("set-chord-structure", "[<uint32 0>]")
    time.sleep(0.3)


if __name__ == "__main__":
    from e2e.helpers import clean_screenshots
    clean_screenshots()
    ctx = TestContext()
    test_mute_toggle(ctx)
    ctx.summarize()
