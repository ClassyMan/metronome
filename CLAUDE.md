# GNOME Metronome — Claude Code Notes

## Build & Run

```bash
# Configure (first time or after meson.build changes)
meson setup build --prefix="$HOME/.local"

# Build
ninja -C build

# Install to ~/.local
ninja -C build install

# Run (needs GSettings schema path)
GSETTINGS_SCHEMA_DIR="$HOME/.local/share/glib-2.0/schemas" \
  GTK_A11Y=atspi \
  ~/.local/bin/metronome
```

- `GTK_A11Y=atspi` is required for E2E tests (AT-SPI accessibility)
- Meson generates `src/config.rs` from template — must run `meson setup` before `cargo test`
- Blueprint `.blp` files compile to `.ui` XML during the build; edit `.blp` not `.ui`

## E2E Test Framework

```bash
# One-time setup
python3 -m venv --system-site-packages .venv
.venv/bin/pip install dogtail

# Run tests (app must be running with GTK_A11Y=atspi)
DISPLAY=:1 .venv/bin/python3 e2e_test.py
```

### Architecture
- `e2e/helpers.py` — `TestContext` class, `dbus_action()`, `clean_screenshots()`
- `e2e/test_*.py` — one file per functional requirement
- `e2e_test.py` — runner that executes all test modules
- `e2e_clean.sh` — safe screenshot cleanup (no rm -rf)
- Screenshots land in `e2e_screenshots/` at 30% resolution

### Interaction Strategy
- **Buttons/tabs**: `dogtail do_action(0)` via AT-SPI — no coordinate guessing
- **ComboRows/fretboard**: D-Bus GAction activation (`gdbus call`) — bypasses UI popover entirely
- **Never use rawinput.click()** for widget interaction — coordinates are unreliable across HiDPI, window managers, and multi-monitor setups

### AT-SPI Prerequisites
- `org.gnome.desktop.interface toolkit-accessibility` must be `true`
- `at-spi2-registryd` must be running — if the a11y tree is empty, restart it: `kill $(pgrep at-spi2-registryd)` (it auto-restarts)
- The GNOME session must be logged in (not locked/logged out) — AT-SPI doesn't work without an active session
- AdwComboRow popup items don't expose names via AT-SPI — use D-Bus GActions to set values instead

### D-Bus Test Actions
Registered as `app.*` actions (exported on `com.adrienplazas.Metronome`):
- `set-chord-structure` `(u)` — set chord structure dropdown by index (0=None, 1=Triad, ...)
- `tap-fret` `(uu)` — simulate tapping string_idx, fret on the fretboard

Invoke from shell:
```bash
gdbus call --session \
  --dest com.adrienplazas.Metronome \
  --object-path /com/adrienplazas/Metronome \
  --method org.freedesktop.Application.ActivateAction \
  'set-chord-structure' '[<uint32 1>]' '{}'
```

## Theme System
- Themes define CSS `@define-color` variables (e.g., `accent_bg_color`)
- Fretboard reads accent color via `style_context().lookup_color("accent_bg_color")` (deprecated but no GTK4 replacement exists)
- Built-in themes in `data/resources/themes/*.json`; user themes in `~/.local/share/metronome/themes/`

## Key Architecture
- GSettings for all persistent config
- Timer thread communicates via `mpsc` channel (`TimerCommand` enum)
- Scales page: ComboRow dropdowns for Root/Family/Mode/ChordStructure
- Fretboard: custom `MtrFretboard` widget with GTK4 snapshot rendering
- Chord playback: GStreamer Play API with staggered note scheduling and randomized velocity/timing
- Tap tempo: 3-second sliding window with averaged intervals

## Branch Stack
Features stack on each other. The `main` branch has everything merged. New features should branch off `main`.
