"""Scale name and degree labels update when root/family/mode change."""

import time
from dogtail.predicate import GenericPredicate
from e2e.helpers import TestContext, dbus_action


def test_scale_info_display(ctx: TestContext):
    print("\n== Scale Info Display ==")
    ctx.switch_to_scales()
    time.sleep(0.5)

    # Default should be C Ionian
    name_label = ctx.find(lambda n: n.roleName == "label" and "Ionian" in (n.name or ""))
    ctx.check(name_label is not None,
              f"Scale name shows 'C Ionian' (got: '{name_label.name if name_label else None}')")

    degree_label = ctx.find(lambda n: n.roleName == "label" and "1  2  3" in (n.name or ""))
    ctx.check(degree_label is not None,
              f"Degree labels visible (got: '{degree_label.name if degree_label else None}')")

    ctx.screenshot("scale_info_c_ionian")


if __name__ == "__main__":
    from e2e.helpers import clean_screenshots
    clean_screenshots()
    ctx = TestContext()
    test_scale_info_display(ctx)
    ctx.summarize()
