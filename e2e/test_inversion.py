"""Chord inversion button cycles Root → Inv 1 → Inv 2 → Root for a triad."""

import time
from e2e.helpers import TestContext, dbus_action


def test_inversion_cycling(ctx: TestContext):
    print("\n== Inversion Cycling ==")
    ctx.switch_to_scales()
    time.sleep(0.5)

    # Set up: Triad chord, tap a note
    dbus_action("set-chord-structure", "[<uint32 1>]")
    time.sleep(0.3)
    dbus_action("tap-fret", "[<(uint32 2, uint32 3)>]")
    time.sleep(0.5)

    inv = ctx.find(lambda n: n.roleName == "push button"
                   and n.name in ("Root", "Inv 1", "Inv 2", "Inv 3"))
    ctx.check(inv is not None and inv.showing,
              f"Inversion button visible (name: '{inv.name if inv else None}')")

    if not inv or not inv.showing:
        return

    ctx.check(inv.name == "Root", f"Starts at Root (got: '{inv.name}')")

    ctx.click(inv)
    time.sleep(0.3)
    ctx.screenshot("inv_1")
    inv = ctx.find(lambda n: n.roleName == "push button" and "Inv" in (n.name or ""))
    ctx.check(inv is not None and inv.name == "Inv 1",
              f"After 1st click: Inv 1 (got: '{inv.name if inv else None}')")

    ctx.click(inv)
    time.sleep(0.3)
    ctx.screenshot("inv_2")
    inv = ctx.find(lambda n: n.roleName == "push button" and "Inv" in (n.name or ""))
    ctx.check(inv is not None and inv.name == "Inv 2",
              f"After 2nd click: Inv 2 (got: '{inv.name if inv else None}')")

    ctx.click(inv)
    time.sleep(0.3)
    ctx.screenshot("inv_root_again")
    inv = ctx.find(lambda n: n.roleName == "push button" and n.name == "Root")
    ctx.check(inv is not None, "After 3rd click: back to Root")

    # Cleanup
    dbus_action("set-chord-structure", "[<uint32 0>]")
    time.sleep(0.3)


if __name__ == "__main__":
    from e2e.helpers import clean_screenshots
    clean_screenshots()
    ctx = TestContext()
    test_inversion_cycling(ctx)
    ctx.summarize()
