# Iced UI Rewrite — Status

## Branch & PR
- Branch: `feature/iced-rewrite` (PR #5)
- Run: `cargo run --bin metronome-iced`
- Test: `cargo test --bin metronome-iced` (179 passing)

## What's done
- **Metronome page**: BPM, beats-per-bar, tap tempo, play/stop, volume, rodio click audio, subscription timer
- **Scales page**: root/family/mode/chord/inversion/pentatonic dropdowns, Canvas fretboard with scale notes and degree labels
- **Tab player page**: full transport (play/pause/stop/seek/loop/tone/tempo/volume), file dialog (rfd), Canvas tab strip + fretboard with glow animation, audio thread integration via Arc<Mutex<Vec>> polling
- **Settings**: JSON persistence at ~/.config/metronome/settings.json
- **All business logic shared**: parsers, audio thread, scale data, chord builder unchanged

## Remaining for full parity
- Guitar sample playback on scales page (rodio + include_bytes for guitar_*.ogg)
- Scrolling for tab strip and fretboard canvases (currently fixed-width)
- Keyboard shortcuts (Space=play, T=tap, arrows=seek)
- Theming (dark is hardcoded)
- Count-in, tempo ramp
- Remove GTK4 binary and deps

## File layout
```
src/main_iced.rs              — entry point
src/ui/mod.rs                 — App, Page enum, message routing, settings lifecycle
src/ui/metronome_page.rs      — MetronomePage (16 tests)
src/ui/scales_page.rs         — ScalesPage (9 tests)
src/ui/tab_player_page.rs     — TabPlayerPage (10 tests)
src/ui/fretboard_canvas.rs    — Scales fretboard (Canvas)
src/ui/tab_strip_canvas.rs    — Tab notation strip (Canvas)
src/ui/tab_fretboard_canvas.rs — Tab active-note fretboard (Canvas)
src/ui/audio.rs               — ClickPlayer (rodio)
src/ui/settings.rs            — JSON persistence (5 tests)
```
