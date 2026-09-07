//! Offline analysis: STFT metrics plus a labeled spectrogram PNG for agents.

mod font;
mod plot;

use crate::error::{Error, Result};
use crate::preset::{factory_ids, load_factory, Preset};
use crate::render::{peak, render, rms, ExportParams, RenderParams, DEFAULT_OUTPUT_DIR};
use crate::resolve_frequency;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const SPECTROGRAM_WIDTH: u32 = 1280;
pub const SPECTROGRAM_HEIGHT: u32 = 800;

const MIN_SAMPLES: usize = 256;
const DB_FLOOR: f32 = -80.0;
const PITCH_MIN_HZ: f32 = 30.0;
const PITCH_MAX_HZ: f32 = 800.0;
const PITCH_MIN_CONF: f32 = 0.35;

#[derive(Clone, Debug, Default)]
pub struct AnalyzeOpts {
    pub source: Option<String>,
    pub preset_id: Option<String>,
    pub preset_name: Option<String>,
    pub description: Option<String>,
    pub intent: Option<String>,
    pub frequency_hz: Option<f64>,
    pub midi_note: Option<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BandEnergy {
    pub sub_20_80: f32,
    pub bass_80_250: f32,
    pub mid_250_2000: f32,
    pub high_2000_plus: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct EnergyAtFrac {
    pub t20: f32,
    pub t50: f32,
    pub t85: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct PitchTrack {
    pub method: &'static str,
    pub start_hz: f32,
    pub end_hz: f32,
    pub drop_semitones: f32,
    pub confidence: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct SpectralPeak {
    pub hz: f32,
    pub db: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImagePaths {
    pub spectrogram: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct AnalysisReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    pub sample_rate: u32,
    pub duration_secs: f64,
    pub sample_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_hz: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_note: Option<u8>,
    pub peak: f32,
    pub rms: f32,
    pub crest_db: f32,
    pub attack_ms: f32,
    pub tail_rms: f32,
    pub spectral_centroid_hz: f32,
    pub spectral_rolloff_hz: f32,
    pub spectral_flatness: f32,
    pub band_energy: BandEnergy,
    pub energy_at_frac: EnergyAtFrac,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch: Option<PitchTrack>,
    pub peaks_hz: Vec<SpectralPeak>,
    pub category_hints: Vec<String>,
    pub stft_nfft: usize,
    pub stft_hop: usize,
    pub images: ImagePaths,
}

#[derive(Debug)]
pub struct Analysis {
    pub report: AnalysisReport,
    samples: Vec<f32>,
    sample_rate: u32,
    /// Per-frame power spectrum, length nfft/2+1.
    power: Vec<Vec<f32>>,
    nfft: usize,
}

#[derive(Clone, Debug)]
pub struct AnalyzeWriteReport {
    pub preset_id: String,
    pub png: PathBuf,
    pub json: PathBuf,
    pub wav: Option<PathBuf>,
}

#[derive(Debug)]
pub struct BatchAnalyzeResult {
    pub written: Vec<AnalyzeWriteReport>,
    pub failures: Vec<(String, String)>,
}

impl BatchAnalyzeResult {
    pub fn into_result(self) -> Result<Vec<AnalyzeWriteReport>> {
        if self.failures.is_empty() {
            Ok(self.written)
        } else {
            Err(Error::BatchFailed {
                failures: self.failures,
            })
        }
    }
}

pub fn default_png_path(preset_id: &str) -> PathBuf {
    PathBuf::from(DEFAULT_OUTPUT_DIR).join(format!("{preset_id}.png"))
}

pub fn default_json_path(preset_id: &str) -> PathBuf {
    PathBuf::from(DEFAULT_OUTPUT_DIR).join(format!("{preset_id}.json"))
}

/// STFT metrics from a mono buffer. Keeps the spectrogram matrix for PNG output.
pub fn analyze_buffer(samples: &[f32], sample_rate: u32, opts: &AnalyzeOpts) -> Result<Analysis> {
    if samples.len() < MIN_SAMPLES {
        return Err(Error::InvalidParam {
            message: format!(
                "analyze needs at least {MIN_SAMPLES} samples, got {}",
                samples.len()
            ),
        });
    }
    if !(8_000..=192_000).contains(&sample_rate) {
        return Err(Error::InvalidParam {
            message: format!("sample_rate must be 8000-192000, got {sample_rate}"),
        });
    }
    if samples.iter().any(|s| !s.is_finite()) {
        return Err(Error::InvalidParam {
            message: "analyze buffer contains NaN/Inf".into(),
        });
    }

    let nfft = pick_nfft(samples.len());
    let hop = (nfft / 4).max(1);
    let power = stft_power(samples, nfft, hop);
    let sr = sample_rate as f32;
    let mean = mean_power(&power);
    let pk = peak(samples);
    let r = rms(samples);
    let crest_db = if r > 1e-12 {
        20.0 * (pk / r).log10()
    } else {
        0.0
    };

    let band_energy = BandEnergy {
        sub_20_80: band_frac(&mean, sr, nfft, 20.0, 80.0),
        bass_80_250: band_frac(&mean, sr, nfft, 80.0, 250.0),
        mid_250_2000: band_frac(&mean, sr, nfft, 250.0, 2_000.0),
        high_2000_plus: band_frac(&mean, sr, nfft, 2_000.0, sr * 0.5),
    };

    let duration_secs = samples.len() as f64 / f64::from(sample_rate);
    let category_hints = opts
        .preset_id
        .as_deref()
        .map(category_hints_for_id)
        .unwrap_or_default();

    let report = AnalysisReport {
        source: opts.source.clone(),
        preset_id: opts.preset_id.clone(),
        preset_name: opts.preset_name.clone(),
        description: opts.description.clone().filter(|s| !s.is_empty()),
        intent: opts.intent.clone().filter(|s| !s.is_empty()),
        sample_rate,
        duration_secs,
        sample_count: samples.len(),
        frequency_hz: opts.frequency_hz,
        midi_note: opts.midi_note,
        peak: pk,
        rms: r,
        crest_db,
        attack_ms: attack_ms(samples, sample_rate),
        tail_rms: window_rms(samples, 0.90, 0.10),
        spectral_centroid_hz: spectral_centroid(&mean, sr, nfft),
        spectral_rolloff_hz: spectral_rolloff(&mean, sr, nfft, 0.85),
        spectral_flatness: spectral_flatness(&mean),
        band_energy,
        energy_at_frac: EnergyAtFrac {
            t20: window_rms(samples, 0.20, 0.05),
            t50: window_rms(samples, 0.50, 0.05),
            t85: window_rms(samples, 0.85, 0.05),
        },
        pitch: pitch_track(samples, sample_rate, nfft, hop),
        peaks_hz: spectral_peaks(&mean, sr, nfft),
        category_hints,
        stft_nfft: nfft,
        stft_hop: hop,
        images: ImagePaths {
            spectrogram: String::new(),
        },
    };

    Ok(Analysis {
        report,
        samples: samples.to_vec(),
        sample_rate,
        power,
        nfft,
    })
}

/// Render a preset and analyze the buffer (does not write files).
pub fn analyze_preset(
    preset_id: &str,
    preset: &Preset,
    export: &ExportParams,
    intent: Option<&str>,
) -> Result<(Vec<f32>, Analysis)> {
    let frequency_hz = resolve_frequency(preset, export.note, export.hz)?;
    let duration_secs = export.duration.unwrap_or(preset.default_duration);
    let samples = render(
        preset,
        &RenderParams {
            frequency_hz,
            duration_secs,
            velocity: export.velocity,
            sample_rate: export.sample_rate,
        },
    )?;
    let analysis = analyze_buffer(
        &samples,
        export.sample_rate,
        &AnalyzeOpts {
            source: None,
            preset_id: Some(preset_id.to_string()),
            preset_name: Some(preset.name.clone()),
            description: Some(preset.description.clone()),
            intent: intent.map(str::to_string),
            frequency_hz: Some(frequency_hz),
            midi_note: export.note.or(Some(preset.default_note)),
        },
    )?;
    Ok((samples, analysis))
}

pub fn write_analysis_bundle(analysis: &Analysis, png: &Path, json: &Path) -> Result<()> {
    analysis.write_png(png)?;
    analysis.write_json(json, png)
}

impl Analysis {
    pub fn write_png(&self, path: &Path) -> Result<()> {
        plot::write_spectrogram_png(self, path)
    }

    pub fn write_json(&self, path: &Path, spectrogram_path: &Path) -> Result<()> {
        ensure_parent(path)?;
        let mut report = self.report.clone();
        report.images.spectrogram = spectrogram_path.to_string_lossy().replace('\\', "/");
        let body = serde_json::to_vec_pretty(&report).map_err(|e| Error::InvalidParam {
            message: format!("json encode: {e}"),
        })?;
        let mut file = fs::File::create(path).map_err(|e| Error::Io {
            path: Some(path.to_path_buf()),
            source: e,
        })?;
        file.write_all(&body).map_err(|e| Error::Io {
            path: Some(path.to_path_buf()),
            source: e,
        })?;
        file.write_all(b"\n").map_err(|e| Error::Io {
            path: Some(path.to_path_buf()),
            source: e,
        })?;
        Ok(())
    }
}

/// Render every factory preset in memory and write `dir/<id>.png` + `.json`.
pub fn analyze_all_factory(
    output_dir: &Path,
    export: &ExportParams,
    intent: Option<&str>,
) -> Result<BatchAnalyzeResult> {
    fs::create_dir_all(output_dir).map_err(|e| Error::Io {
        path: Some(output_dir.to_path_buf()),
        source: e,
    })?;

    let mut written = Vec::new();
    let mut failures = Vec::new();
    for id in factory_ids() {
        let png = output_dir.join(format!("{id}.png"));
        let json = output_dir.join(format!("{id}.json"));
        match load_factory(id)
            .and_then(|preset| analyze_preset(id, &preset, export, intent))
            .and_then(|(_, analysis)| {
                write_analysis_bundle(&analysis, &png, &json)?;
                Ok(AnalyzeWriteReport {
                    preset_id: id.to_string(),
                    png,
                    json,
                    wav: None,
                })
            }) {
            Ok(report) => written.push(report),
            Err(err) => failures.push((id.to_string(), err.to_string())),
        }
    }
    Ok(BatchAnalyzeResult { written, failures })
}

pub fn category_hints_for_id(id: &str) -> Vec<String> {
    if id.starts_with("bd-") {
        vec![
            "kick one-shot".into(),
            "often strong 20-80 Hz".into(),
            "often pitch drop".into(),
        ]
    } else if id.starts_with("sd-") {
        vec![
            "snare one-shot".into(),
            "mid/high body, not a sub kick".into(),
        ]
    } else if id.starts_with("bs-")
        || matches!(
            id,
            "sub-bass" | "growl-bass" | "reese-mid" | "supersaw-bass" | "bp-growl"
        )
    {
        vec!["bass".into(), "low/mid energy".into()]
    } else if id.starts_with("ld-")
        || matches!(
            id,
            "lead-fm-pluck" | "stab-fm-fifth" | "stab-fm-major" | "filter-pluck" | "stab-pluck"
        )
    {
        vec![
            "lead / held tone".into(),
            "energy should remain near t=0.85".into(),
        ]
    } else if id.starts_with("fx-") || matches!(id, "fm-riser" | "zap" | "hp-air") {
        vec![
            "fx / sweep / noise".into(),
            "pitch or centroid often moves".into(),
        ]
    } else if id.starts_with("pc-") || matches!(id, "cp-house" | "glass-hit" | "metallic-hit") {
        vec![
            "short percussion".into(),
            "little sub unless thud/foley".into(),
        ]
    } else if id.starts_with("dr-") {
        vec![
            "drone hold ~16s".into(),
            "energy at t=0.85 should remain".into(),
        ]
    } else if id.starts_with("pf-") {
        vec![
            "fresh pad hold".into(),
            "not a sub bed; energy above ~150 Hz".into(),
        ]
    } else if id.starts_with("ps-") {
        vec![
            "sparkle pad hold".into(),
            "high partials, not a kick".into(),
        ]
    } else if id.starts_with("pl-") {
        vec!["short pluck".into(), "fast decay, not a 16s pad".into()]
    } else if id.starts_with("ep-") {
        vec![
            "electric piano".into(),
            "integer tines 2x/3x, not a bell".into(),
        ]
    } else {
        Vec::new()
    }
}

fn pick_nfft(n: usize) -> usize {
    if n >= 8192 {
        4096
    } else if n >= 4096 {
        2048
    } else if n >= 2048 {
        1024
    } else if n >= 1024 {
        512
    } else {
        256
    }
}

fn stft_power(samples: &[f32], nfft: usize, hop: usize) -> Vec<Vec<f32>> {
    let bins = nfft / 2 + 1;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(nfft);
    let denom = (nfft.saturating_sub(1)).max(1) as f32;
    let window: Vec<f32> = (0..nfft)
        .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / denom).cos())
        .collect();
    let mut scratch = vec![Complex::new(0.0, 0.0); nfft];
    let mut frames = Vec::new();
    let mut start = 0;
    while start + nfft <= samples.len() {
        for i in 0..nfft {
            scratch[i] = Complex::new(samples[start + i] * window[i], 0.0);
        }
        fft.process(&mut scratch);
        let mut frame = vec![0.0f32; bins];
        for (i, p) in frame.iter_mut().enumerate() {
            *p = scratch[i].norm_sqr();
        }
        frames.push(frame);
        start += hop;
    }
    if frames.is_empty() {
        for i in 0..nfft {
            let s = samples.get(i).copied().unwrap_or(0.0);
            scratch[i] = Complex::new(s * window[i], 0.0);
        }
        fft.process(&mut scratch);
        let mut frame = vec![0.0f32; bins];
        for (i, p) in frame.iter_mut().enumerate() {
            *p = scratch[i].norm_sqr();
        }
        frames.push(frame);
    }
    frames
}

fn mean_power(frames: &[Vec<f32>]) -> Vec<f32> {
    let bins = frames[0].len();
    let mut acc = vec![0.0f64; bins];
    for frame in frames {
        for (i, &p) in frame.iter().enumerate() {
            acc[i] += f64::from(p);
        }
    }
    let n = frames.len() as f64;
    acc.into_iter().map(|x| (x / n) as f32).collect()
}

fn bin_hz(bin: usize, sr: f32, nfft: usize) -> f32 {
    bin as f32 * sr / nfft as f32
}

fn band_frac(mean: &[f32], sr: f32, nfft: usize, lo: f32, hi: f32) -> f32 {
    let mut num = 0.0f64;
    let mut den = 0.0f64;
    for (i, &p) in mean.iter().enumerate() {
        let hz = bin_hz(i, sr, nfft);
        if hz < 20.0 {
            continue;
        }
        den += f64::from(p);
        if hz >= lo && hz < hi {
            num += f64::from(p);
        }
    }
    if den < 1e-20 {
        0.0
    } else {
        (num / den) as f32
    }
}

fn spectral_centroid(mean: &[f32], sr: f32, nfft: usize) -> f32 {
    let mut num = 0.0f64;
    let mut den = 0.0f64;
    for (i, &p) in mean.iter().enumerate().skip(1) {
        let hz = f64::from(bin_hz(i, sr, nfft));
        num += hz * f64::from(p);
        den += f64::from(p);
    }
    if den < 1e-20 {
        0.0
    } else {
        (num / den) as f32
    }
}

fn spectral_rolloff(mean: &[f32], sr: f32, nfft: usize, fraction: f32) -> f32 {
    let den: f64 = mean.iter().skip(1).map(|p| f64::from(*p)).sum();
    if den < 1e-20 {
        return 0.0;
    }
    let target = den * f64::from(fraction);
    let mut acc = 0.0;
    for (i, &p) in mean.iter().enumerate().skip(1) {
        acc += f64::from(p);
        if acc >= target {
            return bin_hz(i, sr, nfft);
        }
    }
    bin_hz(mean.len().saturating_sub(1), sr, nfft)
}

fn spectral_flatness(mean: &[f32]) -> f32 {
    const EPS: f64 = 1e-20;
    let mut logsum = 0.0;
    let mut sum = 0.0;
    let mut n = 0usize;
    for &p in mean.iter().skip(1) {
        let x = f64::from(p) + EPS;
        logsum += x.ln();
        sum += x;
        n += 1;
    }
    if n == 0 {
        return 0.0;
    }
    let geo = (logsum / n as f64).exp();
    let arith = sum / n as f64;
    if arith < EPS {
        0.0
    } else {
        (geo / arith) as f32
    }
}

fn spectral_peaks(mean: &[f32], sr: f32, nfft: usize) -> Vec<SpectralPeak> {
    let mut peaks = Vec::new();
    if mean.len() < 3 {
        return peaks;
    }
    let max_p = mean.iter().copied().fold(0.0f32, f32::max).max(1e-20);
    for i in 1..mean.len() - 1 {
        let hz = bin_hz(i, sr, nfft);
        if hz < 20.0 {
            continue;
        }
        if mean[i] > mean[i - 1] && mean[i] >= mean[i + 1] {
            let db = 10.0 * (mean[i] / max_p).max(1e-20).log10();
            if db > DB_FLOOR + 12.0 {
                peaks.push(SpectralPeak { hz, db });
            }
        }
    }
    peaks.sort_by(|a, b| b.db.partial_cmp(&a.db).unwrap_or(std::cmp::Ordering::Equal));
    peaks.truncate(8);
    peaks
}

fn attack_ms(samples: &[f32], sample_rate: u32) -> f32 {
    let pk = peak(samples);
    if pk < 1e-8 {
        return 0.0;
    }
    let thresh = pk * 0.9;
    for (i, &s) in samples.iter().enumerate() {
        if s.abs() >= thresh {
            return i as f32 / sample_rate as f32 * 1000.0;
        }
    }
    0.0
}

fn window_rms(samples: &[f32], center_frac: f64, half_frac: f64) -> f32 {
    let n = samples.len();
    if n == 0 {
        return 0.0;
    }
    let half = ((half_frac * n as f64).round() as usize).max(1);
    let center = ((center_frac * n as f64).round() as usize).min(n.saturating_sub(1));
    let start = center.saturating_sub(half);
    let end = (center + half).min(n);
    if end <= start {
        return 0.0;
    }
    rms(&samples[start..end])
}

fn pitch_track(samples: &[f32], sample_rate: u32, _nfft: usize, _hop: usize) -> Option<PitchTrack> {
    let sr = sample_rate as f32;
    let peak_a = peak(samples);
    if peak_a < 1e-6 {
        return None;
    }
    let mut voiced: Vec<(f32, f32, f32)> = Vec::new();

    let fine_end = ((sr * 0.2) as usize).min(samples.len());
    let win_early = ((sr * 0.04) as usize).clamp(256, 2048);
    let hop_early = (win_early / 4).max(64);
    let mut start = 0;
    while start + win_early <= fine_end {
        let frame = &samples[start..start + win_early];
        if rms(frame) >= peak_a * 0.04 {
            if let Some((hz, conf)) = pitch_from_frame(frame, sr) {
                voiced.push((start as f32 / sr, hz, conf));
            }
        }
        start += hop_early;
    }

    let win_body = ((sr * 0.08) as usize).clamp(512, 4096);
    let hop_body = (win_body / 2).max(128);
    let body_limit = samples.len().saturating_sub(samples.len() / 8);
    start = fine_end.saturating_sub(win_body / 2);
    while start + win_body <= body_limit {
        let frame = &samples[start..start + win_body];
        if rms(frame) >= peak_a * 0.04 {
            if let Some((hz, conf)) = pitch_from_frame(frame, sr) {
                voiced.push((start as f32 / sr, hz, conf));
            }
        }
        start += hop_body;
    }

    if voiced.len() < 3 {
        return None;
    }
    let dur = samples.len() as f32 / sr;
    let early: Vec<_> = voiced
        .iter()
        .copied()
        .filter(|(t, _, _)| *t <= (dur * 0.2).max(0.08))
        .collect();
    let late: Vec<_> = voiced
        .iter()
        .copied()
        .filter(|(t, _, _)| *t >= dur * 0.35 && *t <= dur * 0.8)
        .collect();
    let (start_hz, end_hz) = if !early.is_empty() && !late.is_empty() {
        (median_hz(&early), median_hz(&late))
    } else {
        let n = voiced.len();
        let third = (n / 3).max(1);
        (median_hz(&voiced[..third]), median_hz(&voiced[n - third..]))
    };
    if start_hz <= 0.0 || end_hz <= 0.0 {
        return None;
    }
    let confidence = voiced.iter().map(|(_, _, c)| *c).sum::<f32>() / voiced.len() as f32;
    if confidence < PITCH_MIN_CONF {
        return None;
    }
    Some(PitchTrack {
        method: "autocorr",
        start_hz,
        end_hz,
        drop_semitones: 12.0 * (start_hz / end_hz).log2(),
        confidence,
    })
}

fn median_hz(frames: &[(f32, f32, f32)]) -> f32 {
    let mut v: Vec<f32> = frames.iter().map(|(_, hz, _)| *hz).collect();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v[v.len() / 2]
}

fn pitch_from_frame(frame: &[f32], sr: f32) -> Option<(f32, f32)> {
    let min_lag = (sr / PITCH_MAX_HZ).round() as usize;
    let max_lag = (sr / PITCH_MIN_HZ).round() as usize;
    if min_lag < 2 {
        return None;
    }
    let max_lag = max_lag.min(frame.len().saturating_sub(16));
    if max_lag <= min_lag + 2 {
        return None;
    }
    let energy: f32 = frame.iter().map(|x| x * x).sum();
    if energy < 1e-8 {
        return None;
    }
    let mean_pow = energy / frame.len() as f32;
    let mut corr = vec![0.0f32; max_lag + 1];
    for lag in min_lag..=max_lag {
        let n = frame.len() - lag;
        let mut acc = 0.0f32;
        for i in 0..n {
            acc += frame[i] * frame[i + lag];
        }
        corr[lag] = acc / n as f32;
    }
    let mut best_lag = 0usize;
    let mut best = 0.0f32;
    for lag in (min_lag + 1)..max_lag {
        let c = corr[lag];
        if c > corr[lag - 1] && c >= corr[lag + 1] && c > best {
            best = c;
            best_lag = lag;
        }
    }
    if best_lag == 0 {
        return None;
    }
    let conf = (best / mean_pow.max(1e-12)).clamp(0.0, 1.0);
    if conf < PITCH_MIN_CONF {
        return None;
    }
    Some((sr / best_lag as f32, conf))
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| Error::Io {
                path: Some(parent.to_path_buf()),
                source: e,
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, sr: u32, secs: f64) -> Vec<f32> {
        let n = (secs * f64::from(sr)).round() as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sr as f32;
                (std::f32::consts::TAU * freq * t).sin() * 0.5
            })
            .collect()
    }

    #[test]
    fn sine_centroid_near_440() {
        let sr = 22_050;
        let buf = sine(440.0, sr, 0.5);
        let a = analyze_buffer(&buf, sr, &AnalyzeOpts::default()).unwrap();
        assert!(
            (a.report.spectral_centroid_hz - 440.0).abs() < 30.0,
            "centroid {}",
            a.report.spectral_centroid_hz
        );
        assert!(
            a.report.spectral_flatness < 0.2,
            "flatness {}",
            a.report.spectral_flatness
        );
    }

    #[test]
    fn too_short_is_error() {
        let err = analyze_buffer(&[0.1; 10], 44_100, &AnalyzeOpts::default()).unwrap_err();
        match err {
            Error::InvalidParam { message } => assert!(message.contains("256")),
            other => panic!("unexpected {other}"),
        }
    }
}
