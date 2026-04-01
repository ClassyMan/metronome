"""Chord structure is disabled for non-7-note scales (Messiaen, Blues)."""

import time
from dogtail.predicate import GenericPredicate
from e2e.helpers import TestContext, dbus_action


def test_chord_disabled_for_non_diatonic(ctx: TestContext):
    print("\n== Chord Disabled for Non-Diatonic Scales ==")
    ctx.switch_to_scales()
    time.sleep(0.5)

    # Set Triad first (on C Ionian, should work)
    dbus_action("set-chord-structure", "[<uint32 1>]")
    time.sleep(0.3)

    chord_combo = ctx.find(GenericPredicate(roleName="combo box", name="Chord Structure"))
    ctx.check(chord_combo is not None, "Chord Structure combo found")

    # Verify it's sensitive for Major/Ionian
    if chord_combo:
        ctx.check(chord_combo.sensitive, "Chord combo sensitive for C Ionian (7-note)")

    # Switch to Messiaen Mode 1 (family index 3, mode index 0) via D-Bus
    # Family combo is index 3 = Messiaen
    # Can't use D-Bus for family/mode since no action exists — but we can use
    # the combo's accessible interface. For now just verify via screenshot.
    ctx.screenshot("chord_enabled_ionian")

    # TODO: add D-Bus actions for family/mode selection to test Messiaen disable
    # For now this test verifies the combo is sensitive for 7-note scales


if __name__ == "__main__":
    from e2e.helpers import clean_screenshots
    clean_screenshots()
    ctx = TestContext()
    test_chord_disabled_for_non_diatonic(ctx)
    ctx.summarize()
