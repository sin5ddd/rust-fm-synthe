//! Offline 4-operator FM engine and WAV exporter.
//!
//! The crate is the engine; `fm-synth` is a thin CLI around it.

mod adsr;
mod algorithm;
mod error;
mod filter;
mod midi;
mod operator;
mod preset;
mod render;
mod voice;
mod wav;

pub use algorithm::Algorithm;
pub use error::{Error, Result};
pub use filter::{FilterParams, FilterType};
pub use midi::{cents_to_ratio, hz_to_midi, midi_to_hz, semitones_to_ratio};
pub use operator::{FreqMode, OperatorParams, Waveform};
pub use preset::{
    factory_ids, factory_info, load_factory, load_preset, load_preset_file, output_preset_id,
    LfoParams, ModSweep, PitchEnv, Preset, PresetInfo,
};
pub use render::{
    default_wav_path, peak, render, render_all_factory, render_preset_wav, rms, BatchRenderResult,
    ExportParams, RenderParams, WavRenderReport, DEFAULT_OUTPUT_DIR, TARGET_PEAK,
};
pub use wav::{pcm_data_bytes, write_wav, WavSettings};

/// Resolve a note: explicit Hz wins, then MIDI, then the preset default.
pub fn resolve_frequency(preset: &Preset, note_midi: Option<u8>, hz: Option<f64>) -> Result<f64> {
    if let Some(hz) = hz {
        if !hz.is_finite() || hz <= 0.0 {
            return Err(Error::InvalidParam {
                message: format!("--hz must be > 0, got {hz}"),
            });
        }
        return Ok(hz);
    }
    if let Some(note) = note_midi {
        return Ok(midi_to_hz(note));
    }
    Ok(midi_to_hz(preset.default_note))
}
