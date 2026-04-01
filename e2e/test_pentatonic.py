"""Pentatonic variant toggle cycles through Off → Pent 1 → Pent 2 → Off."""

import time
from dogtail.predicate import GenericPredicate
from e2e.helpers import TestContext


def test_pentatonic_toggle(ctx: TestContext):
    print("\n== Pentatonic Toggle ==")
    ctx.switch_to_scales()
    time.sleep(0.5)

    pent = ctx.find(GenericPredicate(roleName="push button", name="Pent"))
    ctx.check(pent is not None and pent.showing,
              "Pent button visible for C Ionian (not in chord mode)")

    ctx.click(pent)
    time.sleep(0.3)
    ctx.screenshot("pent_1_active")
    pent = ctx.find(lambda n: n.roleName == "push button" and "Pent" in (n.name or ""))
    ctx.check(pent is not None and "1" in pent.name,
              f"Shows 'Pent 1' (got: '{pent.name if pent else None}')")

    ctx.click(pent)
    time.sleep(0.3)
    ctx.screenshot("pent_2_active")
    pent = ctx.find(lambda n: n.roleName == "push button" and "Pent" in (n.name or ""))
    ctx.check(pent is not None and "2" in pent.name,
              f"Shows 'Pent 2' (got: '{pent.name if pent else None}')")

    ctx.click(pent)
    time.sleep(0.3)
    ctx.screenshot("pent_off")
    pent = ctx.find(GenericPredicate(roleName="push button", name="Pent"))
    ctx.check(pent is not None, "Back to 'Pent' (full scale)")


if __name__ == "__main__":
    from e2e.helpers import clean_screenshots
    clean_screenshots()
    ctx = TestContext()
    test_pentatonic_toggle(ctx)
    ctx.summarize()
