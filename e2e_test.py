#!/usr/bin/env python3
"""Run all E2E functional tests.

Usage:
    DISPLAY=:1 .venv/bin/python3 e2e_test.py
"""

from e2e.helpers import TestContext, clean_screenshots
from e2e.test_pentatonic import test_pentatonic_toggle
from e2e.test_chord_mode import test_chord_mode
from e2e.test_inversion import test_inversion_cycling
from e2e.test_mute import test_mute_toggle
from e2e.test_scale_info import test_scale_info_display
from e2e.test_non_diatonic import test_chord_disabled_for_non_diatonic
from e2e.test_tab_player import (
    test_tab_player_page_accessible,
    test_tab_player_transport_visible,
    test_load_gp7_file,
    test_load_gp5_file,
)

if __name__ == "__main__":
    clean_screenshots()
    ctx = TestContext()

    test_pentatonic_toggle(ctx)
    test_chord_mode(ctx)
    test_inversion_cycling(ctx)
    test_mute_toggle(ctx)
    test_scale_info_display(ctx)
    test_chord_disabled_for_non_diatonic(ctx)
    test_tab_player_page_accessible(ctx)
    test_tab_player_transport_visible(ctx)
    test_load_gp7_file(ctx)
    test_load_gp5_file(ctx)

    ctx.summarize()
