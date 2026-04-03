# Iced UI Rewrite — Resume Notes

## Branch & PR
- Branch: `feature/iced-rewrite` (PR #5)
- WIP stashed: `git stash pop` to restore

## What's done
- App shell with 3-page tab navigation (Metronome / Scales / Tab Player)
- **Metronome page**: full state machine, rodio click audio, subscription-based timer (16 tests)
- **Scales page**: state machine, Canvas fretboard with scale notes/degree labels/pentatonic dimming (9 tests)
- **Tab player page**: state machine written but **not compiling** — stashed mid-edit
- `FretboardCanvas`: reusable Canvas widget (used by scales, will adapt for tab player)
- `ClickPlayer`: rodio audio for metronome clicks (include_bytes .ogg)
- 164 tests passing at last clean commit

## Exact resume steps
1. `git checkout feature/iced-rewrite && git stash pop`
2. In `tab_player_page.rs`: add `PollBeats` arm to the `update()` match — drain `self.beat_receiver` Arc<Mutex<Vec>> and dispatch the last beat index via `OnBeat`
3. In `src/ui/mod.rs`: re-add `mod tab_player_page;` and wire `TabPlayer` variant into App (struct field, update match, view match, subscription batch) — the stash reverted this
4. Build, fix compile errors
5. Then: Canvas widgets for tab strip and tab fretboard
6. Then: `rfd` crate for native file dialog
7. Then: serde JSON settings persistence (replacing GSettings)
8. Then: theming
9. Then: remove GTK4 binary and deps once parity reached

## Iced 0.14 gotchas
- `iced::application()` boot param is `Fn() -> State`, not `Fn() -> (State, Task)`
- `Subscription::map()` closures **cannot capture variables** — use poll pattern or `Subscription::with`
- No `vertical_space()` — use `Space::new().height(N)`
- `canvas::Program` must be on a struct holding the cache, not on the cache itself
- `text()` needs owned String in view functions (lifetime issues with local &str)

## File layout
```
src/main_iced.rs          — entry point
src/ui/mod.rs             — App struct, Page enum, tab nav, message routing
src/ui/metronome_page.rs  — MetronomePage + tests
src/ui/scales_page.rs     — ScalesPage + tests
src/ui/tab_player_page.rs — TabPlayerPage (WIP, doesn't compile)
src/ui/fretboard_canvas.rs — reusable Canvas fretboard
src/ui/audio.rs           — ClickPlayer (rodio)
```

Business logic shared with GTK binary (unchanged):
`gp5_parser`, `gp7_parser`, `tab_midi`, `tab_models`, `tab_audio_thread`,
`fluidsynth_ffi`, `scale_data`, `chord_builder`, `recent_files`

## Audio thread integration pattern
The tab player wires to the existing `TabAudioThread` (FluidSynth + GStreamer)
by replacing `glib::MainContext::invoke()` with a shared `Arc<Mutex<Vec<usize>>>`.
The audio thread pushes beat indices, the Iced subscription polls at 60fps and
drains the vec into `OnBeat` messages.
