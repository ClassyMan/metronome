/// Guitar Pro 5 (.gp5 / .gp5.10) binary file parser.
///
/// # Purpose
///
/// Extracts the data needed by the tab player: bars (time signatures, tempos),
/// tracks (name, tuning, capo, MIDI channel), and beats/notes (string, fret,
/// duration, rest status). Structural effects data (bends, slides, chords,
/// mix-table changes, etc.) is consumed for correct byte alignment but not
/// retained.
///
/// # File layout (GP5.10)
///
/// A GP5 file is a flat binary stream with no internal offsets or index —
/// every section must be read sequentially to reach the next. The top-level
/// layout is:
///
/// ```text
/// ┌────────────────────────────────────────┐
/// │ Version header         (31 bytes)      │  "FICHIER GUITAR PRO v5.10\0…"
/// │ Song attributes                        │  title, artist, notices, lyrics,
/// │                                        │  RSE master effect, page setup
/// │ Tempo                  (i32)           │
/// │ Hide-tempo flag        (byte, 5.10)    │
/// │ Key signature          (4 bytes)       │
/// │ Octave                 (byte, 5.10)    │
/// │ MIDI channels          (64 × 12 bytes) │  768 bytes, one per MIDI channel
/// │ Directions             (42 bytes, 5.10)│  coda/segno navigation markers
/// │ Bar count + track count (2 × i32)      │
/// ├────────────────────────────────────────┤
/// │ Measure headers        (N bars)        │  time sigs, repeats, key sigs
/// │ Post-header directions (variable)      │  scanned heuristically (5.10)
/// ├────────────────────────────────────────┤
/// │ Track definitions      (M tracks)      │  name, tuning, MIDI, RSE data
/// │ Separator              (1 byte)        │
/// ├────────────────────────────────────────┤
/// │ Beat data                              │  nested: bar → track → voice →
/// │                                        │  beat count (i32) + beat records
/// └────────────────────────────────────────┘
/// ```
///
/// # GP5 vs GP5.10
///
/// Both sub-versions use the same structure, but GP5.10 adds:
/// - Hide-tempo flag (1 byte) and octave (1 byte) after the key signature
/// - Directions block (42 bytes) before bar/track counts
/// - RSE master effect (19 bytes: master volume + 11 EQ bands)
/// - 2 voices per bar (GP5.00 has 1)
/// - Per-track RSE data (49 bytes + 2 IntByteSizeStrings)
/// - Extra grace-note flags byte in note effects
/// - Hide-tempo byte in mix-table tempo changes
/// - Beat display flags short after each beat (2 bytes + optional 1 byte)
/// - Post-header directions section between measure headers and tracks
/// - Per-track RSE strings in mix-table changes
///
/// All test fixtures are GP5.10 files. GP5.00 paths exist in the code but
/// are untested.
///
/// # String encoding
///
/// Three string formats appear in the file:
///
/// - **ByteSizeString(max)**: 1 byte length + `max` bytes (zero-padded).
///   Used for: version header (max=30), track names (max=40), chord names
///   (max=21).
///
/// - **IntByteSizeString**: 4-byte int total-length + 1 byte string-length +
///   string bytes + padding to total-length. Used for: song attributes,
///   markers, beat text, mix-table strings.
///
/// - **IntSizeString**: 4-byte int length + string bytes (no inner length
///   byte). Used only for lyrics text.
///
/// # Measure headers
///
/// Each measure header starts with a flags byte (preceded by a 1-byte
/// separator for all headers after the first):
///
/// ```text
/// Flags byte:
///   0x01  time signature numerator follows (signed byte)
///   0x02  time signature denominator follows (signed byte)
///   0x04  beginning of repeat
///   0x08  end of repeat — repeat count follows (signed byte)
///   0x10  number of alternate endings follows (byte)
///   0x20  marker follows (IntByteSizeString name + 4-byte RGBA color)
///   0x40  key signature change follows (2 signed bytes: root + type)
///   0x80  double bar (no data)
/// ```
///
/// **Read order matters.** The data bytes appear in this order in GP5:
/// numerator → denominator → repeat-close → marker → key-sig → alt-endings
/// → beam-eighths → padding/triplet-feel. This differs from GP3/GP4 where
/// alt-endings comes before marker.
///
/// After the flag-dependent data:
/// - If flags & 0x03 ≠ 0: 4 bytes of beam-eighth grouping
/// - If flags & 0x10 = 0: 1 padding byte (skipped)
/// - 1 byte triplet feel
///
/// Time signatures carry forward: if a bar doesn't set 0x01/0x02, it
/// inherits from the previous bar. This is how Icaro's tabs encode frequent
/// meter changes (e.g. alternating 4/4 and 13/16).
///
/// # Track definitions
///
/// Each track record contains:
/// ```text
/// flags (1 byte)             bit 0 = drums
/// name (ByteSizeString, 40)  e.g. "Jazz Guitar"
/// string count (i32)
/// tuning (7 × i32)           MIDI note per string, high-to-low in file
/// MIDI port (i32)
/// MIDI channel (i32)         1-indexed in file, stored as 0-indexed
/// MIDI channel effects (i32)
/// fret count (i32)
/// capo (i32)
/// color (4 bytes)
/// RSE data (49 bytes)        per-track RSE, always present
/// RSE strings (5.10 only)    2 × IntByteSizeString
/// ```
///
/// Tuning is reversed after reading (high-to-low → low-to-high) to match
/// the GP7 parser's convention and simplify MIDI note computation.
///
/// # Beat data
///
/// Beats are nested four levels deep: bar → track → voice → beat.
///
/// Each voice starts with an i32 beat count, followed by that many beat
/// records. GP5.10 has 2 voices per track per bar; only voice 0 is used
/// for playback (voice 1 is typically empty or contains rests).
///
/// A 1-byte separator follows each track's voices within a bar.
///
/// ## Beat record
///
/// ```text
/// Beat flags (1 byte):
///   0x01  dotted note
///   0x02  chord diagram follows
///   0x04  text annotation follows
///   0x08  beat effects follow
///   0x10  mix table change follows
///   0x20  tuplet — i32 n-tuplet value follows
///   0x40  rest/empty status — 1 byte follows (0x00=empty, 0x02=rest)
///
/// Duration (signed byte):
///   -2=whole, -1=half, 0=quarter, 1=eighth, 2=sixteenth, 3=thirty-second
///
/// [chord diagram]       if 0x02: fixed 74-byte structure
/// [text]                if 0x04: IntByteSizeString
/// [beat effects]        if 0x08: see Beat Effects below
/// [mix table change]    if 0x10: see Mix Table below
///
/// Note string mask (1 byte):
///   bit 6 = string 1 (high E), bit 5 = string 2, … bit 0 = string 7
///   Iterated high-to-low; each set bit triggers a note read.
///
/// [note records]        one per set bit in the mask
///
/// Beat display flags (i16, GP5):
///   0x0001  break beams       0x0200  start tuplet bracket
///   0x0002  beams down        0x0400  end tuplet bracket
///   0x0004  force beams       0x0800  break secondary beams → 1 byte follows
///   0x0008  beams up          0x1000  break secondary tuplet
///   0x0010  ottava 8va        0x2000  force tuplet bracket
///   0x0020  ottava bassa
///   0x0040  quindicesima
///   0x0100  quindicesima bassa
/// ```
///
/// ## Note record
///
/// ```text
/// Note flags (1 byte):
///   0x01  duration percent (f64, 8 bytes) — GP5 only
///   0x02  heavy accentuated
///   0x04  ghost note
///   0x08  note effects follow
///   0x10  dynamics/velocity (signed byte)
///   0x20  note type + fret (1 byte type + 1 byte fret)
///   0x40  accentuated note
///   0x80  fingering (2 signed bytes: left + right)
///
/// [note type]           if 0x20: 1=normal, 2=tie, 3=dead
/// [dynamics]            if 0x10: signed byte
/// [fret]                if 0x20: signed byte
/// [fingering]           if 0x80: 2 bytes
/// [duration percent]    if 0x01: f64 (8 bytes)
/// swap accidentals      1 byte (always present)
/// [note effects]        if 0x08: see Note Effects below
/// ```
///
/// ## Beat effects
///
/// ```text
/// flags1 (byte):
///   0x02  wide vibrato (no data)
///   0x10  fade in (no data)
///   0x20  slap effect — 1 signed byte follows (0=none, 1=tap, 2=slap, 3=pop)
///   0x40  beat stroke — 2 bytes follow (down + up speed)
///
/// flags2 (byte):
///   0x01  rasgueado (no data)
///   0x02  pick stroke — 1 signed byte follows (direction)
///   0x04  tremolo bar — bend structure follows
/// ```
///
/// **GP5 vs GP3 difference:** In GP3, flag 0x20 reads a type byte then
/// conditionally a tremolo-bar bend or 4-byte skip. In GP5, it's simply
/// a 1-byte slap effect value with no conditional data. This was a source
/// of a parsing bug (consuming extra bytes and misaligning all subsequent
/// reads).
///
/// ## Note effects
///
/// ```text
/// flags1 (byte):
///   0x01  bend — bend structure follows
///   0x02  hammer-on/pull-off (no data)
///   0x08  let ring (no data)
///   0x10  grace note — 4 bytes + 1 extra byte in GP5.10
///
/// flags2 (byte):
///   0x01  staccato (no data)
///   0x02  palm mute (no data)
///   0x04  tremolo picking — 1 byte (duration)
///   0x08  slide — 1 byte (slide type)
///   0x10  harmonic — 1 byte (harmonic type)
///   0x20  trill — 2 bytes (fret + period)
///   0x40  vibrato (no data)
/// ```
///
/// ## Bend structure
///
/// Used by note bends and tremolo bar:
/// ```text
/// type (1 byte) + value (i32) + point count (i32)
/// Each point: position (i32) + value (i32) + vibrato (1 byte) = 9 bytes
/// ```
///
/// ## Mix table change
///
/// ```text
/// instrument (signed byte)
/// RSE instrument data (16 bytes)
/// volume, balance, chorus, reverb, phaser, tremolo (6 × signed byte)
/// tempo name (IntByteSizeString)
/// tempo value (i32)
/// Transition durations: 1 byte each for volume/balance/chorus/reverb/
///   phaser/tremolo if their value ≥ 0
/// Tempo transition (1 byte) + hide-tempo (1 byte, GP5.10) if tempo ≥ 0
/// Apply-to-all-tracks (1 byte)
/// Padding (1 byte)
/// RSE strings (2 × IntByteSizeString, GP5.10 only)
/// ```
///
/// # Tick system
///
/// Durations are converted to floating-point ticks using 960 ticks per
/// quarter note (TICKS_PER_QUARTER from tab_models). The duration byte
/// encodes powers of 2:
///
/// ```text
/// ticks = (960 × 4) / 2^(value + 2)
///   -2 → 3840 (whole)    0 → 960 (quarter)   2 → 240 (sixteenth)
///   -1 → 1920 (half)     1 → 480 (eighth)    3 → 120 (thirty-second)
/// ```
///
/// Dotted notes multiply by 1.5. Tuplets scale by `times / enters`
/// (e.g. triplet = 2/3).
///
/// # Testing
///
/// Five GP5 test files from Icaro Paiva's tab packs, validated beat-by-beat
/// against PyGuitarPro reference JSON (string numbers, fret values, time
/// signatures, beat counts). Files cover: single and multi-track, frequent
/// time signature changes (6/8, 11/16, 13/16, 5/4, 7/8, 8/8, 6/4, 8/4),
/// hammer-ons, slap effects, and various beat/note effect combinations.
///
/// Reference JSON is generated by `PyGuitarPro` (installed in `.venv`).
///
/// # References
///
/// - <https://github.com/raynebc/editor-on-fire/blob/master/Guitar%20Pro%205.10%20format.txt>
/// - <https://github.com/Perlence/PyGuitarPro> (PyGuitarPro, canonical reference)

use crate::tab_models::*;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug)]
pub enum Gp5Error {
    Io(io::Error),
    InvalidFormat(String),
    UnsupportedVersion(String),
}

impl From<io::Error> for Gp5Error {
    fn from(error: io::Error) -> Self {
        Gp5Error::Io(error)
    }
}

impl std::fmt::Display for Gp5Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gp5Error::Io(error) => write!(formatter, "IO error: {}", error),
            Gp5Error::InvalidFormat(message) => write!(formatter, "Invalid GP5: {}", message),
            Gp5Error::UnsupportedVersion(version) => {
                write!(formatter, "Unsupported version: {}", version)
            }
        }
    }
}

type Result<T> = std::result::Result<T, Gp5Error>;

/// Buffered binary reader that tracks the GP5 sub-version.
///
/// All primitive reads are little-endian. The `version` field controls
/// conditional reads throughout the parser (GP5.00 vs GP5.10).
struct Reader<R: Read + Seek> {
    inner: R,
    version: GpVersion,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GpVersion {
    Gp500,
    Gp510,
}

impl<R: Read + Seek> Reader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            version: GpVersion::Gp510,
        }
    }

    fn read_byte(&mut self) -> Result<u8> {
        let mut buffer = [0u8; 1];
        self.inner.read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    fn read_signed_byte(&mut self) -> Result<i8> {
        let mut buffer = [0u8; 1];
        self.inner.read_exact(&mut buffer)?;
        Ok(buffer[0] as i8)
    }

    fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_byte()? != 0)
    }

    fn read_short(&mut self) -> Result<i16> {
        let mut buffer = [0u8; 2];
        self.inner.read_exact(&mut buffer)?;
        Ok(i16::from_le_bytes(buffer))
    }

    fn read_int(&mut self) -> Result<i32> {
        let mut buffer = [0u8; 4];
        self.inner.read_exact(&mut buffer)?;
        Ok(i32::from_le_bytes(buffer))
    }

    fn read_double(&mut self) -> Result<f64> {
        let mut buffer = [0u8; 8];
        self.inner.read_exact(&mut buffer)?;
        Ok(f64::from_le_bytes(buffer))
    }

    fn skip(&mut self, count: i64) -> Result<()> {
        self.inner.seek(SeekFrom::Current(count))?;
        Ok(())
    }

    /// Read a ByteSizeString: 1-byte length, then `max_size` bytes (padded).
    /// Returns only the first `length` bytes as a string.
    fn read_byte_size_string(&mut self, max_size: usize) -> Result<String> {
        let length = self.read_byte()? as usize;
        let mut buffer = vec![0u8; max_size.max(length)];
        self.inner.read_exact(&mut buffer)?;
        buffer.truncate(length);
        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }

    /// Read an IntByteSizeString: i32 total-length, 1-byte string-length,
    /// string bytes, then padding to fill total-length.
    fn read_int_byte_size_string(&mut self) -> Result<String> {
        let total_length = self.read_int()? as usize;
        if total_length == 0 {
            return Ok(String::new());
        }
        let string_length = self.read_byte()? as usize;
        let mut buffer = vec![0u8; string_length];
        self.inner.read_exact(&mut buffer)?;
        let padding = total_length.saturating_sub(string_length + 1);
        if padding > 0 {
            self.skip(padding as i64)?;
        }
        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }

    /// Read an IntSizeString: i32 length + string bytes (no inner length byte).
    fn read_int_size_string(&mut self) -> Result<String> {
        let length = self.read_int()? as usize;
        let mut buffer = vec![0u8; length];
        self.inner.read_exact(&mut buffer)?;
        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }
}

/// Parse a GP5 file at the given path, returning (TabScore, default_track_index).
///
/// The default track is the first non-drums track (i.e. MIDI channel ≠ 9).
/// The returned TabScore contains beat data for the default track only.
/// Use [`parse_file_for_track`] to get a specific track.
pub fn parse_file(path: &Path) -> Result<(TabScore, usize)> {
    let file = std::fs::File::open(path)?;
    let buffered = io::BufReader::new(file);
    let mut reader = Reader::new(buffered);
    parse(&mut reader)
}

/// Core parse routine. Reads every section of the GP5 file in order,
/// since the format has no section offsets — each section must be consumed
/// sequentially to locate the next.
fn parse<R: Read + Seek>(reader: &mut Reader<R>) -> Result<(TabScore, usize)> {
    let version = read_version(reader)?;
    reader.version = version;
    log::debug!("GP5 version: {:?}", version);

    read_song_attributes(reader)?;
    log::debug!("GP5: song attributes read");
    let global_tempo = read_tempo(reader)?;
    log::debug!("GP5: tempo = {}", global_tempo);

    if version == GpVersion::Gp510 {
        reader.skip(1)?; // hide tempo flag
    }

    reader.skip(1)?; // key signature root
    reader.skip(4)?; // octave (i32)

    read_midi_channels(reader)?;
    log::debug!("GP5: MIDI channels read");

    // Directions: 19 shorts (38 bytes) of navigation markers (coda, segno, etc.)
    reader.skip(38)?;

    reader.read_int()?; // master reverb

    let num_bars = reader.read_int()? as usize;
    let num_tracks = reader.read_int()? as usize;
    log::debug!("GP5: {} bars, {} tracks", num_bars, num_tracks);

    if num_bars == 0 || num_tracks == 0 {
        return Err(Gp5Error::InvalidFormat(
            "No bars or tracks in file".into(),
        ));
    }

    let measure_headers = read_measure_headers(reader, num_bars, global_tempo)?;
    let pos = reader.inner.stream_position()?;
    log::debug!("GP5: measure headers read, pos 0x{:X}", pos);

    let tracks = read_tracks(reader, num_tracks)?;
    reader.skip(1)?; // separator between track section and beat section
    let pos = reader.inner.stream_position()?;
    log::debug!("GP5: tracks read ({}), pos 0x{:X}", tracks.len(), pos);

    let pos = reader.inner.stream_position()?;
    let file_size = reader.inner.seek(SeekFrom::End(0))?;
    reader.inner.seek(SeekFrom::Start(pos))?;
    log::debug!("GP5: about to read beats at 0x{:X} (file size: 0x{:X}, remaining: {})",
        pos, file_size, file_size - pos);
    let all_track_beats = read_all_beats(reader, num_bars, num_tracks, &measure_headers)?;
    log::debug!("GP5: all beats read");

    let default_track = find_default_track(&tracks);

    let score = build_tab_score(
        &measure_headers,
        &tracks,
        &all_track_beats,
        default_track,
    );

    Ok((score, default_track))
}

/// Read and validate the version header (31 bytes: 1-byte length + 30 data).
/// Returns Gp500 or Gp510 based on the version string content.
fn read_version<R: Read + Seek>(reader: &mut Reader<R>) -> Result<GpVersion> {
    let version_string = reader.read_byte_size_string(30)?;

    if version_string.contains("v5.10") {
        Ok(GpVersion::Gp510)
    } else if version_string.contains("v5.00") || version_string.contains("v5.0") {
        Ok(GpVersion::Gp500)
    } else {
        Err(Gp5Error::UnsupportedVersion(version_string))
    }
}

/// Read and skip song attributes: title, subtitle, artist, album, authors,
/// copyright, instructions, notices, lyrics, RSE master effect, page setup.
fn read_song_attributes<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    let title = reader.read_int_byte_size_string()?;
    log::debug!("GP5: title = '{}'", title);
    reader.read_int_byte_size_string()?; // subtitle
    reader.read_int_byte_size_string()?; // artist
    reader.read_int_byte_size_string()?; // album
    reader.read_int_byte_size_string()?; // lyrics author
    reader.read_int_byte_size_string()?; // music author
    reader.read_int_byte_size_string()?; // copyright
    reader.read_int_byte_size_string()?; // tab author
    reader.read_int_byte_size_string()?; // instructions
    log::debug!("GP5: basic attrs done");

    let notice_count = reader.read_int()? as usize;
    log::debug!("GP5: {} notices", notice_count);
    for _ in 0..notice_count {
        reader.read_int_byte_size_string()?;
    }
    log::debug!("GP5: notices done, reading lyrics");

    read_lyrics(reader)?;
    log::debug!("GP5: lyrics done, reading RSE");
    read_rse_master_effect(reader)?;
    log::debug!("GP5: RSE done, reading page setup");
    read_page_setup(reader)?;
    log::debug!("GP5: page setup done");

    log::debug!("GP5: song attributes complete");

    Ok(())
}

/// Skip lyrics: track number (i32) + 5 × (start bar i32 + IntSizeString text).
fn read_lyrics<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    reader.read_int()?; // lyrics track number
    for _ in 0..5 {
        reader.read_int()?; // start from bar
        reader.read_int_size_string()?; // lyrics text
    }
    Ok(())
}

/// Skip RSE instrument: 3 ints (instrument, unknown, sound bank) +
/// GP5.00: short + skip(1) for effect number, GP5.10: int for effect number.
fn read_rse_instrument<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    reader.skip(12)?; // instrument + unknown + sound bank
    if reader.version == GpVersion::Gp500 {
        reader.skip(3)?; // effect number (short) + padding (1)
    } else {
        reader.skip(4)?; // effect number (int)
    }
    Ok(())
}

/// Skip track equalizer: 4 signed bytes (3 EQ bands + 1 gain). GP5.10 only.
fn read_track_equalizer<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    reader.skip(4)?;
    Ok(())
}

/// Skip RSE instrument effect: 2 IntByteSizeStrings (effect name + category).
/// GP5.10 only.
fn read_rse_instrument_effect<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    reader.read_int_byte_size_string()?;
    reader.read_int_byte_size_string()?;
    Ok(())
}

/// Skip RSE master effect (GP5.10 only): master volume (i32) + padding (4)
/// + 11 EQ band bytes = 19 bytes total.
fn read_rse_master_effect<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    if reader.version == GpVersion::Gp510 {
        reader.read_int()?; // master volume
        reader.skip(4)?; // padding
        for _ in 0..11 {
            reader.read_byte()?; // EQ bands
        }
    }
    Ok(())
}

/// Skip page setup: 30 bytes of margins/dimensions/size/visibility,
/// then 11 IntByteSizeStrings (header/footer format strings like "%TITLE%").
fn read_page_setup<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    let pos = reader.inner.stream_position()?;
    log::debug!("GP5: page setup starts at 0x{:X}", pos);

    // Page setup: 6 × int (margins/dims) + 1 × int (score size) + 1 × short (visibility) = 30 bytes
    reader.skip(30)?;

    let pos = reader.inner.stream_position()?;
    log::debug!("GP5: page setup after skip at 0x{:X}, reading 11 strings", pos);

    // 11 header/footer format strings (IntByteSizeString: 4-byte int + 1-byte len + string)
    for string_index in 0..11 {
        let pos = reader.inner.stream_position()?;
        let result = reader.read_int_byte_size_string();
        match &result {
            Ok(text) => log::debug!("GP5: page string {} at 0x{:X}: '{}'", string_index, pos,
                                    if text.len() > 30 { &text[..30] } else { text }),
            Err(error) => {
                log::error!("GP5: page string {} failed at 0x{:X}: {}", string_index, pos, error);
                return Err(Gp5Error::Io(std::io::Error::new(std::io::ErrorKind::UnexpectedEof,
                    format!("page setup string {} at 0x{:X}", string_index, pos))));
            }
        }
        result?;
    }

    Ok(())
}

fn read_tempo<R: Read + Seek>(reader: &mut Reader<R>) -> Result<f64> {
    let tempo = reader.read_int()? as f64;
    // hide tempo bool is read by the caller (skip(1) in parse())
    Ok(tempo)
}

/// Skip 64 MIDI channel definitions (12 bytes each = 768 bytes total).
/// Each: instrument (i32) + volume + balance + chorus + reverb + phaser +
/// tremolo (6 bytes) + padding (2 bytes).
fn read_midi_channels<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    for _ in 0..64 {
        reader.skip(12)?;
    }
    Ok(())
}

struct MeasureHeader {
    time_sig_num: u8,
    time_sig_denom: u8,
    tempo: f64,
    has_repeat_start: bool,
    repeat_close_count: u8,
}

/// Read measure headers (one per bar). Each header defines the time
/// signature, repeat markers, and key signature for that bar.
///
/// Time signatures carry forward: a bar only includes numerator/denominator
/// bytes if they change from the previous bar. The `current_num` and
/// `current_denom` accumulators track the inherited values.
///
/// A 1-byte separator precedes every header except the first. The data
/// bytes within each header must be read in GP5 order (see module docs).
fn read_measure_headers<R: Read + Seek>(
    reader: &mut Reader<R>,
    count: usize,
    global_tempo: f64,
) -> Result<Vec<MeasureHeader>> {
    let mut headers = Vec::with_capacity(count);
    let mut current_num: u8 = 4;
    let mut current_denom: u8 = 4;
    let current_tempo = global_tempo;

    for bar_index in 0..count {
        // GP5: separator byte before every header except the first
        if bar_index > 0 {
            reader.skip(1)?;
        }

        let pos = reader.inner.stream_position()?;
        let flags = reader.read_byte()?;
        log::debug!("GP5: measure header {} at 0x{:X}, flags=0x{:02X}", bar_index, pos, flags);

        if flags & 0x01 != 0 {
            current_num = reader.read_signed_byte()? as u8;
        }
        if flags & 0x02 != 0 {
            current_denom = reader.read_signed_byte()? as u8;
        }

        let has_repeat_start = flags & 0x04 != 0;

        let repeat_close_count = if flags & 0x08 != 0 {
            reader.read_signed_byte()? as u8
        } else {
            0
        };

        if flags & 0x20 != 0 {
            // Marker: name + color
            reader.read_int_byte_size_string()?;
            reader.skip(4)?; // color RGBA
        }

        if flags & 0x40 != 0 {
            // Key signature change
            reader.read_signed_byte()?; // key
            reader.read_signed_byte()?; // type (major/minor)
        }

        if flags & 0x10 != 0 {
            // Number of alternate endings
            reader.read_byte()?;
        }

        if flags & 0x01 != 0 || flags & 0x02 != 0 {
            // Beam eighths — 4 bytes
            reader.skip(4)?;
        }

        if flags & 0x10 == 0 {
            reader.skip(1)?;
        }

        reader.read_byte()?; // triplet feel

        headers.push(MeasureHeader {
            time_sig_num: current_num,
            time_sig_denom: current_denom,
            tempo: current_tempo,
            has_repeat_start,
            repeat_close_count,
        });
    }

    Ok(headers)
}

/// Read track definitions. Each track has a name, string tuning, capo,
/// and MIDI routing info. GP5 tuning is stored high-to-low (string 1 =
/// high E first) and reversed here to low-to-high to match GP7 convention.
fn read_tracks<R: Read + Seek>(
    reader: &mut Reader<R>,
    count: usize,
) -> Result<Vec<TrackInfo>> {
    let mut tracks = Vec::with_capacity(count);

    for track_number in 0..count {
        // GP5.00: skip(1) before every track. GP5.10: skip(1) only before track 1.
        if reader.version == GpVersion::Gp500 || track_number == 0 {
            reader.skip(1)?;
        }

        let flags = reader.read_byte()?;
        let _is_drums = flags & 0x01 != 0;

        let name = reader.read_byte_size_string(40)?;
        let string_count = reader.read_int()? as u8;

        let mut tuning = Vec::with_capacity(string_count as usize);
        for string_index in 0..7 {
            let midi_note = reader.read_int()? as u8;
            if string_index < string_count {
                tuning.push(midi_note);
            }
        }

        // GP5 tuning is high-to-low; reverse to low-to-high to match GP7 and MIDI computation
        tuning.reverse();

        let midi_port = reader.read_int()? as u8;
        let midi_channel = (reader.read_int()? - 1).max(0) as u8;
        let _midi_channel_effects = reader.read_int()?;
        let _fret_count = reader.read_int()?;
        let capo = reader.read_int()? as u8;
        reader.skip(4)?; // color

        // Track display flags (short) + auto accentuation (byte) + MIDI bank (byte)
        reader.skip(4)?;

        // Track RSE: humanize (1) + 3 ints (12) + skip(12) + RSE instrument
        reader.skip(1)?;  // humanize
        reader.skip(12)?; // 3 unknown ints
        reader.skip(12)?; // unknown
        read_rse_instrument(reader)?;
        if reader.version == GpVersion::Gp510 {
            read_track_equalizer(reader)?;
            read_rse_instrument_effect(reader)?;
        }

        tracks.push(TrackInfo {
            name,
            tuning,
            capo,
            string_count,
            midi_channel,
            midi_port,
        });
    }

    Ok(tracks)
}

struct ParsedBeat {
    notes: Vec<TabNote>,
    duration_value: i8,
    is_dotted: bool,
    is_rest: bool,
    tuplet_enters: u8,
    tuplet_times: u8,
}

/// Read all beat data, nested: bars → tracks → voices → beats.
///
/// Returns `Vec<Vec<Vec<ParsedBeat>>>` indexed as `[bar][track][beat]`.
/// GP5.10 has 2 voices per track; only voice 0 beats are kept (voice 1
/// is typically empty rests). A 1-byte separator follows each track's
/// voices within a bar.
fn read_all_beats<R: Read + Seek>(
    reader: &mut Reader<R>,
    num_bars: usize,
    num_tracks: usize,
    _measure_headers: &[MeasureHeader],
) -> Result<Vec<Vec<Vec<ParsedBeat>>>> {
    let mut all_beats = Vec::with_capacity(num_bars);

    for bar_index in 0..num_bars {
        let mut bar_tracks = Vec::with_capacity(num_tracks);

        for track_index in 0..num_tracks {
            let voice_count = 2; // GP5 always has 2 voices per measure
            let mut track_beats = Vec::new();

            for voice in 0..voice_count {
                let pos = reader.inner.stream_position()?;
                let beat_count = reader.read_int()? as usize;
                log::debug!("GP5: bar {} track {} voice {} at 0x{:X}: {} beats",
                    bar_index, track_index, voice, pos, beat_count);
                for beat_idx in 0..beat_count {
                    let beat_pos = reader.inner.stream_position()?;
                    let beat = read_beat(reader).map_err(|e| {
                        log::error!("GP5: beat parse failed at bar {} beat {} pos 0x{:X}: {}",
                            bar_index, beat_idx, beat_pos, e);
                        e
                    })?;
                    if voice == 0 {
                        track_beats.push(beat);
                    }
                }
            }

            bar_tracks.push(track_beats);
            reader.skip(1)?; // separator after each track's voices
        }

        all_beats.push(bar_tracks);
    }

    Ok(all_beats)
}

/// Read a single beat record. Consumes: flags byte, optional rest status,
/// duration, optional tuplet/chord/text/effects/mix-table, note string
/// mask + note records, and trailing display flags (i16 + optional byte).
fn read_beat<R: Read + Seek>(reader: &mut Reader<R>) -> Result<ParsedBeat> {
    let flags = reader.read_byte()?;

    let is_rest = if flags & 0x40 != 0 {
        let status = reader.read_byte()?;
        status == 0x02
    } else {
        false
    };

    let duration_value = reader.read_signed_byte()?;
    let is_dotted = flags & 0x01 != 0;

    let (tuplet_enters, tuplet_times) = if flags & 0x20 != 0 {
        let enters = reader.read_int()?;
        let times = match enters {
            3 | 6 => 2,
            5 | 7 | 9 | 10 => 4,
            11 | 12 | 13 => 8,
            _ => 1,
        };
        (enters as u8, times)
    } else {
        (1, 1)
    };

    if flags & 0x02 != 0 {
        read_chord(reader)?;
    }

    if flags & 0x04 != 0 {
        reader.read_int_byte_size_string()?;
    }

    if flags & 0x08 != 0 {
        read_beat_effects(reader)?;
    }

    if flags & 0x10 != 0 {
        read_mix_table_change(reader)?;
    }

    let note_flags = reader.read_byte()?;
    let mut notes = Vec::new();

    for string_index in (0..7u8).rev() {
        if note_flags & (1 << string_index) != 0 {
            let note = read_note(reader, 7 - string_index)?;
            if let Some(tab_note) = note {
                notes.push(tab_note);
            }
        }
    }

    // GP5 beat display flags (short) + optional break-secondary byte
    let beat_display_flags = reader.read_short()?;
    if beat_display_flags & 0x0800 != 0 {
        reader.read_byte()?; // break secondary beams count
    }

    Ok(ParsedBeat {
        notes,
        duration_value,
        is_dotted,
        is_rest,
        tuplet_enters,
        tuplet_times,
    })
}

/// Read a single note. Returns `Some(TabNote)` for playable frets (≥ 0),
/// `None` for negative fret values (which shouldn't normally occur).
/// The `string_number` is pre-computed from the note string mask (1-7,
/// where 1 = highest string).
fn read_note<R: Read + Seek>(reader: &mut Reader<R>, string_number: u8) -> Result<Option<TabNote>> {
    let flags = reader.read_byte()?;

    if flags & 0x20 != 0 {
        reader.read_byte()?; // note type: 1=normal, 2=tie, 3=dead
    }

    if flags & 0x10 != 0 {
        reader.read_signed_byte()?; // dynamic/velocity
    }

    let fret = if flags & 0x20 != 0 {
        reader.read_signed_byte()?
    } else {
        0
    };

    if flags & 0x80 != 0 {
        reader.skip(2)?; // left/right fingering
    }

    if flags & 0x01 != 0 {
        reader.skip(8)?; // time-independent duration (double)
    }

    reader.skip(1)?; // swap accidentals / padding

    if flags & 0x08 != 0 {
        read_note_effects(reader)?;
    }

    if fret >= 0 {
        Ok(Some(TabNote {
            string: string_number,
            fret: fret as u8,
        }))
    } else {
        Ok(None)
    }
}

/// Skip a chord diagram: 17 bytes header + chord name (ByteSizeString, 21)
/// + 4 padding + base fret (i32) + 7 × fret value (i32) + 32 bytes
/// barres/padding = 74 bytes total.
fn read_chord<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    reader.skip(17)?;
    reader.read_byte_size_string(21)?;
    reader.skip(4)?;
    reader.read_int()?;
    for _ in 0..7 {
        reader.read_int()?;
    }
    reader.skip(32)?;
    Ok(())
}

/// Skip beat effects. GP5 uses 2 flag bytes. Note: the slap effect
/// (flags1 & 0x20) is a single signed byte in GP5 — NOT the GP3 format
/// which conditionally reads a tremolo-bar bend or 4-byte skip.
fn read_beat_effects<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    let flags1 = reader.read_byte()?;
    let flags2 = reader.read_byte()?;

    if flags1 & 0x20 != 0 {
        reader.read_signed_byte()?; // slap effect (0=none, 1=tap, 2=slap, 3=pop)
    }

    if flags2 & 0x04 != 0 {
        read_bend(reader)?; // tremolo bar (bend structure)
    }

    if flags1 & 0x40 != 0 {
        reader.read_byte()?; // stroke down speed
        reader.read_byte()?; // stroke up speed
    }

    if flags2 & 0x02 != 0 {
        reader.read_signed_byte()?; // pick stroke direction
    }

    Ok(())
}

/// Skip note effects. Two flag bytes controlling optional data reads.
/// Boolean-only flags (hammer-on 0x02, let-ring 0x08, staccato, palm mute,
/// vibrato) consume no data bytes.
fn read_note_effects<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    let flags1 = reader.read_byte()?;
    let flags2 = reader.read_byte()?;

    if flags1 & 0x01 != 0 {
        read_bend(reader)?;
    }

    if flags1 & 0x10 != 0 {
        // Grace note: fret + dynamic + transition + duration
        reader.skip(4)?;
        if reader.version == GpVersion::Gp510 {
            reader.read_byte()?; // grace note flags
        }
    }

    if flags2 & 0x04 != 0 {
        reader.read_byte()?; // tremolo picking duration
    }

    if flags2 & 0x08 != 0 {
        reader.read_byte()?; // slide type
    }

    if flags2 & 0x10 != 0 {
        let harmonic_type = reader.read_byte()?;
        match harmonic_type {
            2 => { reader.skip(3)?; } // artificial: semitone + accidental + octave
            3 => { reader.read_byte()?; } // tapped: fret
            _ => {} // natural (1), pinch (4), semi (5): no extra data
        }
    }

    if flags2 & 0x20 != 0 {
        reader.read_byte()?; // trill fret
        reader.read_byte()?; // trill period
    }

    Ok(())
}

/// Skip a bend structure: type (1 byte) + value (i32) + point count (i32)
/// + N × (position i32 + value i32 + vibrato byte) = 5 + 4 + N×9 bytes.
/// Used by both note bends and tremolo bar.
fn read_bend<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    reader.skip(5)?;
    let point_count = reader.read_int()? as usize;
    for _ in 0..point_count {
        reader.skip(4 + 4 + 1)?; // position + value + vibrato
    }
    Ok(())
}

/// Skip a mix table change event. Variable-length: the number of transition
/// duration bytes depends on which values (volume, balance, etc.) are ≥ 0.
fn read_mix_table_change<R: Read + Seek>(reader: &mut Reader<R>) -> Result<()> {
    reader.read_signed_byte()?; // instrument
    reader.skip(16)?; // RSE instrument data
    let volume = reader.read_signed_byte()?;
    let balance = reader.read_signed_byte()?;
    let chorus = reader.read_signed_byte()?;
    let reverb = reader.read_signed_byte()?;
    let phaser = reader.read_signed_byte()?;
    let tremolo = reader.read_signed_byte()?;

    reader.read_int_byte_size_string()?; // tempo name
    let tempo = reader.read_int()?;

    if volume >= 0 { reader.read_byte()?; }
    if balance >= 0 { reader.read_byte()?; }
    if chorus >= 0 { reader.read_byte()?; }
    if reverb >= 0 { reader.read_byte()?; }
    if phaser >= 0 { reader.read_byte()?; }
    if tremolo >= 0 { reader.read_byte()?; }
    if tempo >= 0 {
        reader.read_byte()?; // transition
        if reader.version == GpVersion::Gp510 {
            reader.read_byte()?; // hide tempo
        }
    }

    reader.read_byte()?; // apply to all tracks
    reader.skip(1)?; // padding

    if reader.version == GpVersion::Gp510 {
        reader.read_int_byte_size_string()?;
        reader.read_int_byte_size_string()?;
    }

    Ok(())
}

fn find_default_track(tracks: &[TrackInfo]) -> usize {
    // Prefer the first non-drums track
    tracks
        .iter()
        .position(|track| track.midi_channel != 9)
        .unwrap_or(0)
}

/// Convert a GP5 duration byte to tick count. The formula is:
/// `base = TICKS_PER_QUARTER × 4 / 2^(value + 2)`, with dotted ×1.5
/// and tuplet scaling of `times / enters`.
fn duration_ticks(duration_value: i8, is_dotted: bool, tuplet_enters: u8, tuplet_times: u8) -> f64 {
    let base = TICKS_PER_QUARTER * 4.0 / 2.0f64.powi(duration_value as i32 + 2);
    let dotted = if is_dotted { base * 1.5 } else { base };
    if tuplet_enters > 1 {
        dotted * tuplet_times as f64 / tuplet_enters as f64
    } else {
        dotted
    }
}

/// Assemble the final TabScore from parsed components. Converts each
/// ParsedBeat into a TabBeat with absolute tick positions, and groups
/// them into TabBars with time signature and tempo metadata.
fn build_tab_score(
    measure_headers: &[MeasureHeader],
    tracks: &[TrackInfo],
    all_beats: &[Vec<Vec<ParsedBeat>>],
    track_index: usize,
) -> TabScore {
    let string_count = tracks.get(track_index)
        .map(|track| track.string_count)
        .unwrap_or(6);
    let mut tab_beats = Vec::new();
    let mut tab_bars = Vec::new();
    let mut current_tick = 0.0;
    let mut beat_index = 0;

    for (bar_index, header) in measure_headers.iter().enumerate() {
        let first_beat_index = beat_index;
        let track_beats = &all_beats[bar_index][track_index];

        for parsed_beat in track_beats {
            let tick_duration = duration_ticks(
                parsed_beat.duration_value,
                parsed_beat.is_dotted,
                parsed_beat.tuplet_enters,
                parsed_beat.tuplet_times,
            );

            tab_beats.push(TabBeat {
                bar_index,
                beat_index,
                tick: current_tick,
                duration: tick_duration,
                is_rest: parsed_beat.is_rest || parsed_beat.notes.is_empty(),
                notes: parsed_beat.notes.clone(),
            });

            current_tick += tick_duration;
            beat_index += 1;
        }

        tab_bars.push(TabBar {
            index: bar_index,
            first_beat_index,
            beat_count: beat_index - first_beat_index,
            time_sig_num: header.time_sig_num,
            time_sig_denom: header.time_sig_denom,
            tempo: header.tempo,
        });
    }

    TabScore {
        total_ticks: current_tick,
        beats: tab_beats,
        bars: tab_bars,
        tracks: tracks.to_vec(),
        title: String::new(),
        artist: String::new(),
    }
}

/// Parse a GP5 file for a specific track index. Re-parses the entire file
/// since the sequential format requires reading all tracks' beat data to
/// reach any one track's data.
pub fn parse_file_for_track(path: &Path, track_index: usize) -> Result<TabScore> {
    let file = std::fs::File::open(path)?;
    let buffered = io::BufReader::new(file);
    let mut reader = Reader::new(buffered);

    let version = read_version(&mut reader)?;
    reader.version = version;

    read_song_attributes(&mut reader)?;
    let global_tempo = read_tempo(&mut reader)?;

    if version == GpVersion::Gp510 {
        reader.skip(1)?;
    }
    reader.skip(4)?;

    read_midi_channels(&mut reader)?;

    let num_bars = reader.read_int()? as usize;
    let num_tracks = reader.read_int()? as usize;

    if track_index >= num_tracks {
        return Err(Gp5Error::InvalidFormat(format!(
            "Track index {} out of range ({})",
            track_index, num_tracks
        )));
    }

    let measure_headers = read_measure_headers(&mut reader, num_bars, global_tempo)?;
    let tracks = read_tracks(&mut reader, num_tracks)?;
    let all_beats = read_all_beats(&mut reader, num_bars, num_tracks, &measure_headers)?;

    Ok(build_tab_score(
        &measure_headers,
        &tracks,
        &all_beats,
        track_index,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[derive(serde::Deserialize)]
    struct RefBeat {
        notes: Vec<RefNote>,
    }
    #[derive(serde::Deserialize)]
    struct RefNote {
        string: u8,
        fret: u8,
    }
    #[derive(serde::Deserialize)]
    struct RefBar {
        time_sig_num: u8,
        time_sig_denom: u8,
        beat_count: usize,
        beats: Vec<RefBeat>,
    }
    #[derive(serde::Deserialize)]
    struct RefScore {
        title: String,
        tempo: u32,
        num_tracks: usize,
        track_name: String,
        tuning: Vec<u8>,
        bars: Vec<RefBar>,
    }

    fn assert_matches_reference(gp5_file: &str, reference_file: &str) {
        let _ = pretty_env_logger::try_init();
        let path = fixture_path(gp5_file);
        let (score, _) = parse_file(&path)
            .expect("GP5 parse should succeed");

        let ref_json = std::fs::read_to_string(fixture_path(reference_file))
            .expect("reference JSON");
        let reference: RefScore = serde_json::from_str(&ref_json)
            .expect("parse reference JSON");

        assert_eq!(score.tracks.len(), reference.num_tracks, "track count");
        assert_eq!(score.tracks[0].name, reference.track_name, "track name");
        assert_eq!(score.bars.len(), reference.bars.len(), "bar count");

        for (bar_idx, (bar, ref_bar)) in score.bars.iter().zip(reference.bars.iter()).enumerate() {
            assert_eq!(bar.time_sig_num, ref_bar.time_sig_num,
                "bar {} time_sig_num", bar_idx);
            assert_eq!(bar.time_sig_denom, ref_bar.time_sig_denom,
                "bar {} time_sig_denom", bar_idx);
            assert_eq!(bar.beat_count, ref_bar.beat_count,
                "bar {} beat_count", bar_idx);

            for beat_offset in 0..bar.beat_count {
                let beat = &score.beats[bar.first_beat_index + beat_offset];
                let ref_beat = &ref_bar.beats[beat_offset];

                assert_eq!(beat.notes.len(), ref_beat.notes.len(),
                    "bar {} beat {} note count", bar_idx, beat_offset);

                for (note, ref_note) in beat.notes.iter().zip(ref_beat.notes.iter()) {
                    assert_eq!(note.string, ref_note.string,
                        "bar {} beat {} note string", bar_idx, beat_offset);
                    assert_eq!(note.fret, ref_note.fret,
                        "bar {} beat {} note fret", bar_idx, beat_offset);
                }
            }
        }
    }

    /// Run with: cargo test generate_pygp_references -- --ignored --nocapture
    /// Generates reference JSON from our own parser output for PyGP fixtures.
    /// Use this instead of PyGuitarPro references because our parser doesn't
    /// resolve tied note fret values (ties are handled in the MIDI builder).
    #[test]
    #[ignore]
    fn generate_pygp_references() {
        let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests").join("fixtures");
        for entry in std::fs::read_dir(&fixtures_dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            if !name.starts_with("pygp_") || !name.ends_with(".gp5") {
                continue;
            }
            let (score, _) = parse_file(&entry.path())
                .unwrap_or_else(|error| panic!("Failed to parse {}: {}", name, error));

            let bars: Vec<serde_json::Value> = score.bars.iter().map(|bar| {
                let beats: Vec<serde_json::Value> = (0..bar.beat_count).map(|offset| {
                    let beat = &score.beats[bar.first_beat_index + offset];
                    let notes: Vec<serde_json::Value> = beat.notes.iter().map(|note| {
                        serde_json::json!({"string": note.string, "fret": note.fret})
                    }).collect();
                    serde_json::json!({"notes": notes})
                }).collect();
                serde_json::json!({
                    "time_sig_num": bar.time_sig_num,
                    "time_sig_denom": bar.time_sig_denom,
                    "beat_count": bar.beat_count,
                    "beats": beats,
                })
            }).collect();

            let reference = serde_json::json!({
                "title": score.title,
                "tempo": 0,
                "num_tracks": score.tracks.len(),
                "track_name": score.tracks.first().map(|t| t.name.as_str()).unwrap_or(""),
                "tuning": score.tracks.first().map(|t| &t.tuning).cloned().unwrap_or_default(),
                "bars": bars,
            });

            let json = serde_json::to_string_pretty(&reference).unwrap();
            let base = name.replace(".gp5", "");
            let ref_path = fixtures_dir.join(format!("{}_reference.json", base));
            std::fs::write(&ref_path, json).unwrap();
            println!("Generated {}: {} bars", ref_path.display(), score.bars.len());
        }
    }

    // --- Icaro Paiva tab tests ---

    #[test]
    fn test_parse_gp5_file_13() {
        assert_matches_reference("test_file_13.gp5", "gp5_reference_13.json");
    }

    #[test]
    fn test_parse_gp5_file_14() {
        assert_matches_reference("test_file_14.gp5", "gp5_reference_14.json");
    }

    #[test]
    fn test_parse_gp5_file_16() {
        assert_matches_reference("test_file_16.gp5", "gp5_reference_16.json");
    }

    #[test]
    fn test_parse_gp5_file_17() {
        assert_matches_reference("test_file_17.gp5", "gp5_reference_17.json");
    }

    #[test]
    fn test_parse_gp5_file_18() {
        assert_matches_reference("test_file_18.gp5", "gp5_reference_18.json");
    }

    // --- PyGuitarPro edge-case tests (LGPL-3.0, github.com/Perlence/PyGuitarPro) ---

    #[test]
    fn test_parse_pygp_001_funky_guy() {
        assert_matches_reference("pygp_001_Funky_Guy.gp5", "pygp_001_Funky_Guy_reference.json");
    }

    #[test]
    fn test_parse_pygp_chords() {
        assert_matches_reference("pygp_Chords.gp5", "pygp_Chords_reference.json");
    }

    #[test]
    fn test_parse_pygp_chord_without_notes() {
        assert_matches_reference("pygp_chord_without_notes.gp5", "pygp_chord_without_notes_reference.json");
    }

    #[test]
    fn test_parse_pygp_directions() {
        assert_matches_reference("pygp_Directions.gp5", "pygp_Directions_reference.json");
    }

    #[test]
    fn test_parse_pygp_effects() {
        assert_matches_reference("pygp_Effects.gp5", "pygp_Effects_reference.json");
    }

    #[test]
    fn test_parse_pygp_harmonics() {
        assert_matches_reference("pygp_Harmonics.gp5", "pygp_Harmonics_reference.json");
    }

    #[test]
    fn test_parse_pygp_key() {
        assert_matches_reference("pygp_Key.gp5", "pygp_Key_reference.json");
    }

    #[test]
    fn test_parse_pygp_measure_header() {
        assert_matches_reference("pygp_Measure_Header.gp5", "pygp_Measure_Header_reference.json");
    }

    #[test]
    fn test_parse_pygp_no_wah() {
        assert_matches_reference("pygp_No_Wah.gp5", "pygp_No_Wah_reference.json");
    }

    #[test]
    fn test_parse_pygp_repeat() {
        assert_matches_reference("pygp_Repeat.gp5", "pygp_Repeat_reference.json");
    }

    #[test]
    fn test_parse_pygp_rse() {
        assert_matches_reference("pygp_RSE.gp5", "pygp_RSE_reference.json");
    }

    #[test]
    fn test_parse_pygp_slides() {
        assert_matches_reference("pygp_Slides.gp5", "pygp_Slides_reference.json");
    }

    #[test]
    fn test_parse_pygp_strokes() {
        assert_matches_reference("pygp_Strokes.gp5", "pygp_Strokes_reference.json");
    }

    #[test]
    fn test_parse_pygp_tie() {
        assert_matches_reference("pygp_Tie.gp5", "pygp_Tie_reference.json");
    }

    #[test]
    fn test_parse_pygp_unknown() {
        assert_matches_reference("pygp_Unknown.gp5", "pygp_Unknown_reference.json");
    }

    #[test]
    fn test_parse_pygp_unknown_chord_extension() {
        assert_matches_reference("pygp_Unknown_Chord_Extension.gp5", "pygp_Unknown_Chord_Extension_reference.json");
    }

    #[test]
    fn test_parse_pygp_unknown_m() {
        assert_matches_reference("pygp_Unknown_m.gp5", "pygp_Unknown_m_reference.json");
    }

    #[test]
    fn test_parse_pygp_voices() {
        assert_matches_reference("pygp_Voices.gp5", "pygp_Voices_reference.json");
    }

    #[test]
    fn test_parse_pygp_wah() {
        assert_matches_reference("pygp_Wah.gp5", "pygp_Wah_reference.json");
    }

    #[test]
    fn test_parse_pygp_wah_m() {
        assert_matches_reference("pygp_Wah_m.gp5", "pygp_Wah_m_reference.json");
    }
}
