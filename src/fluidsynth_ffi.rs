/// Thin safe wrapper around libfluidsynth C API.
///
/// Only exposes the subset needed for tab player audio synthesis:
/// settings, synth creation, SoundFont loading, MIDI events, and PCM rendering.
/// Matches the Android JNI bridge (fluidsynth_jni.cpp) 1:1.

use std::ffi::CString;
use std::path::Path;

#[allow(non_camel_case_types)]
mod ffi {
    use std::os::raw::{c_char, c_int, c_short, c_uint};

    pub enum fluid_settings_t {}
    pub enum fluid_synth_t {}

    #[link(name = "fluidsynth")]
    extern "C" {
        pub fn new_fluid_settings() -> *mut fluid_settings_t;
        pub fn delete_fluid_settings(settings: *mut fluid_settings_t);

        pub fn fluid_settings_setstr(
            settings: *mut fluid_settings_t,
            name: *const c_char,
            val: *const c_char,
        ) -> c_int;
        pub fn fluid_settings_setnum(
            settings: *mut fluid_settings_t,
            name: *const c_char,
            val: f64,
        ) -> c_int;
        pub fn fluid_settings_setint(
            settings: *mut fluid_settings_t,
            name: *const c_char,
            val: c_int,
        ) -> c_int;

        pub fn new_fluid_synth(settings: *mut fluid_settings_t) -> *mut fluid_synth_t;
        pub fn delete_fluid_synth(synth: *mut fluid_synth_t);

        pub fn fluid_synth_sfload(
            synth: *mut fluid_synth_t,
            filename: *const c_char,
            reset_presets: c_int,
        ) -> c_int;

        pub fn fluid_synth_noteon(
            synth: *mut fluid_synth_t,
            chan: c_int,
            key: c_int,
            vel: c_int,
        ) -> c_int;
        pub fn fluid_synth_noteoff(
            synth: *mut fluid_synth_t,
            chan: c_int,
            key: c_int,
        ) -> c_int;

        pub fn fluid_synth_program_select(
            synth: *mut fluid_synth_t,
            chan: c_int,
            sfont_id: c_uint,
            bank_num: c_uint,
            preset_num: c_uint,
        ) -> c_int;

        pub fn fluid_synth_cc(
            synth: *mut fluid_synth_t,
            chan: c_int,
            num: c_int,
            val: c_int,
        ) -> c_int;

        pub fn fluid_synth_all_notes_off(
            synth: *mut fluid_synth_t,
            chan: c_int,
        ) -> c_int;

        pub fn fluid_synth_write_s16(
            synth: *mut fluid_synth_t,
            len: c_int,
            lout: *mut c_short,
            loff: c_int,
            lincr: c_int,
            rout: *mut c_short,
            roff: c_int,
            rincr: c_int,
        ) -> c_int;
    }
}

pub struct FluidSynthEngine {
    settings: *mut ffi::fluid_settings_t,
    synth: *mut ffi::fluid_synth_t,
}

// FluidSynth is thread-safe for rendering (single writer pattern).
// The audio thread is the sole caller during playback.
unsafe impl Send for FluidSynthEngine {}

impl FluidSynthEngine {
    pub fn new() -> Option<Self> {
        unsafe {
            let settings = ffi::new_fluid_settings();
            if settings.is_null() {
                return None;
            }

            let driver_key = CString::new("audio.driver").unwrap();
            let driver_val = CString::new("file").unwrap();
            ffi::fluid_settings_setstr(settings, driver_key.as_ptr(), driver_val.as_ptr());

            let rate_key = CString::new("synth.sample-rate").unwrap();
            ffi::fluid_settings_setnum(settings, rate_key.as_ptr(), 44100.0);

            let gain_key = CString::new("synth.gain").unwrap();
            ffi::fluid_settings_setnum(settings, gain_key.as_ptr(), 2.0);

            let chorus_key = CString::new("synth.chorus.active").unwrap();
            ffi::fluid_settings_setint(settings, chorus_key.as_ptr(), 0);

            let reverb_key = CString::new("synth.reverb.active").unwrap();
            ffi::fluid_settings_setint(settings, reverb_key.as_ptr(), 1);

            let poly_key = CString::new("synth.polyphony").unwrap();
            ffi::fluid_settings_setint(settings, poly_key.as_ptr(), 64);

            let synth = ffi::new_fluid_synth(settings);
            if synth.is_null() {
                ffi::delete_fluid_settings(settings);
                return None;
            }

            Some(Self { settings, synth })
        }
    }

    pub fn load_soundfont(&self, path: &Path) -> Option<u32> {
        let path_str = CString::new(path.to_str()?).ok()?;
        let sfont_id =
            unsafe { ffi::fluid_synth_sfload(self.synth, path_str.as_ptr(), 1) };
        if sfont_id >= 0 {
            Some(sfont_id as u32)
        } else {
            None
        }
    }

    pub fn note_on(&self, channel: u8, key: u8, velocity: u8) {
        unsafe {
            ffi::fluid_synth_noteon(
                self.synth,
                channel as i32,
                key as i32,
                velocity as i32,
            );
        }
    }

    pub fn note_off(&self, channel: u8, key: u8) {
        unsafe {
            ffi::fluid_synth_noteoff(self.synth, channel as i32, key as i32);
        }
    }

    pub fn program_select(&self, channel: u8, sfont_id: u32, bank: u32, preset: u32) {
        unsafe {
            ffi::fluid_synth_program_select(
                self.synth,
                channel as i32,
                sfont_id,
                bank,
                preset,
            );
        }
    }

    pub fn cc(&self, channel: u8, controller: u8, value: u8) {
        unsafe {
            ffi::fluid_synth_cc(
                self.synth,
                channel as i32,
                controller as i32,
                value as i32,
            );
        }
    }

    pub fn all_notes_off(&self, channel: u8) {
        unsafe {
            ffi::fluid_synth_all_notes_off(self.synth, channel as i32);
        }
    }

    /// Render `frames` stereo samples into `buffer` as interleaved S16LE.
    /// Buffer must have length >= frames * 2.
    pub fn render_s16(&self, buffer: &mut [i16], frames: usize) {
        debug_assert!(buffer.len() >= frames * 2);
        unsafe {
            ffi::fluid_synth_write_s16(
                self.synth,
                frames as i32,
                buffer.as_mut_ptr(),
                0,
                2, // left channel: offset 0, stride 2 (interleaved)
                buffer.as_mut_ptr(),
                1,
                2, // right channel: offset 1, stride 2
            );
        }
    }
}

impl Drop for FluidSynthEngine {
    fn drop(&mut self) {
        unsafe {
            if !self.synth.is_null() {
                ffi::delete_fluid_synth(self.synth);
            }
            if !self.settings.is_null() {
                ffi::delete_fluid_settings(self.settings);
            }
        }
    }
}
