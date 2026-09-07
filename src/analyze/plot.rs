//! Labeled 3-panel PNG: waveform, log-frequency spectrogram, average spectrum.

use super::font;
use super::{Analysis, SPECTROGRAM_HEIGHT, SPECTROGRAM_WIDTH};
use crate::error::{Error, Result};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const W: u32 = SPECTROGRAM_WIDTH;
const H: u32 = SPECTROGRAM_HEIGHT;
const MARGIN_L: u32 = 52;
const MARGIN_R: u32 = 62;
const TITLE_Y: u32 = 8;
const WAVE_TOP: u32 = 36;
const WAVE_H: u32 = 110;
const SPEC_TOP: u32 = 160;
const SPEC_H: u32 = 360;
const AVG_TOP: u32 = 540;
const AVG_H: u32 = 210;
const FMIN: f32 = 20.0;

const BG: [u8; 3] = [12, 14, 20];
const AXIS: [u8; 3] = [180, 188, 200];
const GRID: [u8; 3] = [42, 48, 60];
const WAVE: [u8; 3] = [120, 210, 255];
const TEXT: [u8; 3] = [230, 236, 245];
const MUTED: [u8; 3] = [150, 158, 170];
const FILL: [u8; 3] = [70, 160, 220];

struct Panel {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

struct FreqScale {
    nfft: usize,
    sr: f32,
    fmax: f32,
}

fn label(px: &mut [u8], x: i32, y: i32, s: &str, color: [u8; 3], scale: u32) {
    font::draw_text(
        &mut font::Bitmap {
            px,
            width: W,
            height: H,
        },
        x,
        y,
        s,
        color,
        scale,
    );
}

pub(super) fn write_spectrogram_png(analysis: &Analysis, path: &Path) -> Result<()> {
    super::ensure_parent(path)?;
    let mut px = vec![0u8; (W * H * 3) as usize];
    fill(&mut px, BG);

    let plot_x0 = MARGIN_L;
    let plot_x1 = W - MARGIN_R;
    let plot_w = plot_x1 - plot_x0;
    let sr = analysis.sample_rate as f32;
    let fmax = (sr * 0.5).max(FMIN * 2.0);
    let nfft = analysis.nfft;
    let duration = analysis.samples.len() as f32 / sr;

    let title = title_line(analysis);
    label(&mut px, 10, TITLE_Y as i32, &title, TEXT, 2);

    draw_waveform(
        &mut px,
        plot_x0,
        WAVE_TOP,
        plot_w,
        WAVE_H,
        &analysis.samples,
    );
    label(&mut px, 8, WAVE_TOP as i32 - 2, "waveform", MUTED, 1);

    let spec_panel = Panel {
        x: plot_x0,
        y: SPEC_TOP,
        w: plot_w,
        h: SPEC_H,
    };
    let scale = FreqScale { nfft, sr, fmax };
    draw_spectrogram(&mut px, spec_panel, &analysis.power, &scale);
    draw_freq_axis(&mut px, plot_x0, SPEC_TOP, SPEC_H, fmax);
    draw_time_axis(&mut px, plot_x0, SPEC_TOP + SPEC_H, plot_w, duration);
    draw_colorbar(&mut px, plot_x1 + 6, SPEC_TOP, 12, SPEC_H);

    draw_average_spectrum(
        &mut px,
        Panel {
            x: plot_x0,
            y: AVG_TOP,
            w: plot_w,
            h: AVG_H,
        },
        &analysis.power,
        &scale,
    );
    draw_log_hz_axis(&mut px, plot_x0, AVG_TOP + AVG_H, plot_w, fmax);
    label(
        &mut px,
        plot_x0 as i32 + 6,
        AVG_TOP as i32 + 4,
        "avg spectrum (dB, log Hz)",
        MUTED,
        1,
    );

    let footer = footer_line(analysis);
    label(&mut px, 10, (H - 22) as i32, &footer, TEXT, 1);

    let file = File::create(path).map_err(|e| Error::Io {
        path: Some(path.to_path_buf()),
        source: e,
    })?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), W, H);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| Error::InvalidParam {
        message: format!("png header: {e}"),
    })?;
    writer
        .write_image_data(&px)
        .map_err(|e| Error::InvalidParam {
            message: format!("png encode: {e}"),
        })?;
    Ok(())
}

fn title_line(a: &Analysis) -> String {
    let r = &a.report;
    let id = r
        .preset_id
        .as_deref()
        .or(r.source.as_deref())
        .unwrap_or("buffer");
    let mut s = format!("{id}  {:.2}s  {}Hz", r.duration_secs, r.sample_rate);
    if let Some(note) = r.midi_note {
        s.push_str(&format!("  MIDI {note}"));
    }
    if let Some(hz) = r.frequency_hz {
        s.push_str(&format!("  {hz:.1}Hz"));
    }
    if font::text_width(&s, 2) > W - 16 {
        s.truncate(40);
    }
    s
}

fn footer_line(a: &Analysis) -> String {
    let r = &a.report;
    let pitch = match &r.pitch {
        Some(p) => format!(
            "pitch={:.0}->{:.0}Hz ({:+.1}st)",
            p.start_hz, p.end_hz, p.drop_semitones
        ),
        None => "pitch=n/a".into(),
    };
    format!(
        "centroid={:.0}Hz  flatness={:.3}  {}  sub={:.2} bass={:.2} mid={:.2} high={:.2}  rms={:.3}",
        r.spectral_centroid_hz,
        r.spectral_flatness,
        pitch,
        r.band_energy.sub_20_80,
        r.band_energy.bass_80_250,
        r.band_energy.mid_250_2000,
        r.band_energy.high_2000_plus,
        r.rms
    )
}

fn draw_waveform(px: &mut [u8], x0: u32, y0: u32, w: u32, h: u32, samples: &[f32]) {
    rect(px, x0, y0, w, h, [18, 22, 30]);
    let mid = y0 + h / 2;
    hline(px, x0, x0 + w - 1, mid, GRID);
    if samples.is_empty() || w == 0 {
        return;
    }
    let n = samples.len();
    let peak = samples.iter().fold(1e-6f32, |a, &x| a.max(x.abs()));
    for x in 0..w {
        let i0 = (x as usize * n) / w as usize;
        let i1 = (((x + 1) as usize * n) / w as usize).max(i0 + 1).min(n);
        let slice = &samples[i0..i1];
        let mut mn = 0.0f32;
        let mut mx = 0.0f32;
        for &s in slice {
            mn = mn.min(s);
            mx = mx.max(s);
        }
        let y_min = (mid as i32 - ((mx / peak) * (h / 2 - 2) as f32) as i32)
            .clamp(y0 as i32, (y0 + h - 1) as i32);
        let y_max = (mid as i32 - ((mn / peak) * (h / 2 - 2) as f32) as i32)
            .clamp(y0 as i32, (y0 + h - 1) as i32);
        vline(px, x0 + x, y_min as u32, y_max as u32, WAVE);
    }
}

fn draw_spectrogram(px: &mut [u8], panel: Panel, power: &[Vec<f32>], scale: &FreqScale) {
    let Panel { x: x0, y: y0, w, h } = panel;
    let FreqScale { nfft, sr, fmax } = *scale;
    rect(px, x0, y0, w, h, [8, 8, 12]);
    if power.is_empty() || w == 0 || h == 0 {
        return;
    }
    let n_frames = power.len();
    let bins = power[0].len();
    let mut max_p = 1e-20f32;
    for frame in power {
        for &p in frame {
            if p > max_p {
                max_p = p;
            }
        }
    }
    for x in 0..w {
        let tf = x as f32 / (w - 1).max(1) as f32 * (n_frames.saturating_sub(1)) as f32;
        let t0 = tf.floor() as usize;
        let t1 = (t0 + 1).min(n_frames - 1);
        let tf_frac = tf - t0 as f32;
        for y in 0..h {
            let hz = y_to_hz(y, h, fmax);
            let bin = hz * nfft as f32 / sr;
            if bin < 0.0 {
                continue;
            }
            let b0 = bin.floor() as usize;
            let b1 = (b0 + 1).min(bins - 1);
            let bf = bin - b0 as f32;
            if b0 >= bins {
                continue;
            }
            let p0 = (1.0 - bf) * power[t0][b0] + bf * power[t0][b1];
            let p1 = (1.0 - bf) * power[t1][b0] + bf * power[t1][b1];
            let p = (1.0 - tf_frac) * p0 + tf_frac * p1;
            let db = 10.0 * (p / max_p).max(1e-20).log10();
            let t = ((db - DB_FLOOR_PLOT) / (0.0 - DB_FLOOR_PLOT)).clamp(0.0, 1.0);
            put(px, x0 + x, y0 + y, inferno(t));
        }
    }
}

const DB_FLOOR_PLOT: f32 = -80.0;

fn draw_average_spectrum(px: &mut [u8], panel: Panel, power: &[Vec<f32>], scale: &FreqScale) {
    let Panel { x: x0, y: y0, w, h } = panel;
    let FreqScale { nfft, sr, fmax } = *scale;
    rect(px, x0, y0, w, h, [18, 22, 30]);
    if power.is_empty() {
        return;
    }
    let bins = power[0].len();
    let mut mean = vec![0.0f32; bins];
    for frame in power {
        for (i, &p) in frame.iter().enumerate() {
            mean[i] += p;
        }
    }
    let nf = power.len() as f32;
    for p in &mut mean {
        *p /= nf;
    }
    let max_p = mean.iter().copied().fold(1e-20f32, f32::max);

    for &tick in &FREQ_TICKS {
        if tick < FMIN || tick > fmax {
            continue;
        }
        let x = x0 + hz_to_x(tick, w, fmax);
        vline(px, x, y0, y0 + h - 1, GRID);
    }

    let mut prev: Option<(u32, u32)> = None;
    for x in 0..w {
        let hz = x_to_hz(x, w, fmax);
        let bin = hz * nfft as f32 / sr;
        let b0 = bin.floor() as usize;
        if b0 >= bins {
            continue;
        }
        let b1 = (b0 + 1).min(bins - 1);
        let bf = bin - b0 as f32;
        let p = (1.0 - bf) * mean[b0] + bf * mean[b1];
        let db = 10.0 * (p / max_p).max(1e-20).log10();
        let t = ((db - DB_FLOOR_PLOT) / (0.0 - DB_FLOOR_PLOT)).clamp(0.0, 1.0);
        let y = y0 + h - 1 - ((t * (h - 3) as f32) as u32).min(h - 2);
        if let Some((px0, py0)) = prev {
            line(px, px0, py0, x0 + x, y, FILL);
        }
        prev = Some((x0 + x, y));
    }
}

fn draw_freq_axis(px: &mut [u8], x0: u32, y0: u32, h: u32, fmax: f32) {
    for &tick in &FREQ_TICKS {
        if tick < FMIN || tick > fmax {
            continue;
        }
        let y = y0 + hz_to_y(tick, h, fmax);
        hline(px, x0.saturating_sub(4), x0, y, AXIS);
        let tick_label = hz_label(tick);
        let tw = font::text_width(&tick_label, 1);
        label(
            px,
            x0 as i32 - tw as i32 - 2,
            y as i32 - 3,
            &tick_label,
            MUTED,
            1,
        );
    }
    label(px, 4, y0 as i32, "Hz", MUTED, 1);
}

fn draw_time_axis(px: &mut [u8], x0: u32, y: u32, w: u32, duration: f32) {
    let ticks = time_ticks(duration);
    for t in ticks {
        let frac = (t / duration).clamp(0.0, 1.0);
        let x = x0 + ((frac * (w - 1) as f32) as u32);
        vline(px, x, y, y + 4, AXIS);
        let tick_label = format!("{t:.2}s");
        label(px, x as i32 - 8, y as i32 + 6, &tick_label, MUTED, 1);
    }
}

fn draw_log_hz_axis(px: &mut [u8], x0: u32, y: u32, w: u32, fmax: f32) {
    for &tick in &FREQ_TICKS {
        if tick < FMIN || tick > fmax {
            continue;
        }
        let x = x0 + hz_to_x(tick, w, fmax);
        vline(px, x, y, y + 4, AXIS);
        let tick_label = hz_label(tick);
        label(px, x as i32 - 8, y as i32 + 6, &tick_label, MUTED, 1);
    }
}

fn draw_colorbar(px: &mut [u8], x: u32, y0: u32, w: u32, h: u32) {
    for y in 0..h {
        let t = 1.0 - y as f32 / (h - 1).max(1) as f32;
        let c = inferno(t);
        for dx in 0..w {
            put(px, x + dx, y0 + y, c);
        }
    }
    label(px, x as i32 + w as i32 + 2, y0 as i32, "0dB", MUTED, 1);
    label(
        px,
        x as i32 + w as i32 + 2,
        (y0 + h / 2) as i32,
        "-40",
        MUTED,
        1,
    );
    label(
        px,
        x as i32 + w as i32 + 2,
        (y0 + h - 10) as i32,
        "-80",
        MUTED,
        1,
    );
}

const FREQ_TICKS: [f32; 10] = [
    20.0, 50.0, 100.0, 200.0, 500.0, 1_000.0, 2_000.0, 5_000.0, 10_000.0, 20_000.0,
];

fn hz_label(hz: f32) -> String {
    if hz >= 1000.0 {
        format!("{}k", (hz / 1000.0).round() as i32)
    } else {
        format!("{}", hz.round() as i32)
    }
}

fn y_to_hz(y: u32, h: u32, fmax: f32) -> f32 {
    let t = 1.0 - y as f32 / (h - 1).max(1) as f32;
    (FMIN.ln() + t * (fmax.ln() - FMIN.ln())).exp()
}

fn hz_to_y(hz: f32, h: u32, fmax: f32) -> u32 {
    let t = (hz.max(FMIN).ln() - FMIN.ln()) / (fmax.ln() - FMIN.ln());
    let t = t.clamp(0.0, 1.0);
    ((1.0 - t) * (h - 1) as f32).round() as u32
}

fn x_to_hz(x: u32, w: u32, fmax: f32) -> f32 {
    let t = x as f32 / (w - 1).max(1) as f32;
    (FMIN.ln() + t * (fmax.ln() - FMIN.ln())).exp()
}

fn hz_to_x(hz: f32, w: u32, fmax: f32) -> u32 {
    let t = (hz.max(FMIN).ln() - FMIN.ln()) / (fmax.ln() - FMIN.ln());
    (t.clamp(0.0, 1.0) * (w - 1) as f32).round() as u32
}

fn time_ticks(duration: f32) -> Vec<f32> {
    if duration <= 0.0 {
        return vec![0.0];
    }
    let step = if duration <= 0.5 {
        0.1
    } else if duration <= 2.0 {
        0.25
    } else if duration <= 8.0 {
        1.0
    } else {
        2.0
    };
    let mut t = 0.0;
    let mut out = Vec::new();
    while t <= duration + 1e-6 {
        out.push((t * 1000.0).round() / 1000.0);
        t += step;
        if out.len() > 16 {
            break;
        }
    }
    if out.last().copied().unwrap_or(0.0) < duration * 0.95 {
        out.push(duration);
    }
    out
}

fn inferno(t: f32) -> [u8; 3] {
    const STOPS: [[f32; 3]; 8] = [
        [0.0, 0.0, 4.0],
        [40.0, 11.0, 84.0],
        [101.0, 21.0, 110.0],
        [159.0, 42.0, 99.0],
        [212.0, 72.0, 66.0],
        [245.0, 125.0, 21.0],
        [250.0, 193.0, 39.0],
        [252.0, 255.0, 164.0],
    ];
    let t = t.clamp(0.0, 1.0);
    let x = t * (STOPS.len() - 1) as f32;
    let i = (x.floor() as usize).min(STOPS.len() - 2);
    let f = x - i as f32;
    let a = STOPS[i];
    let b = STOPS[i + 1];
    [
        (a[0] + (b[0] - a[0]) * f) as u8,
        (a[1] + (b[1] - a[1]) * f) as u8,
        (a[2] + (b[2] - a[2]) * f) as u8,
    ]
}

fn fill(px: &mut [u8], c: [u8; 3]) {
    for p in px.chunks_exact_mut(3) {
        p[0] = c[0];
        p[1] = c[1];
        p[2] = c[2];
    }
}

fn put(px: &mut [u8], x: u32, y: u32, c: [u8; 3]) {
    if x >= W || y >= H {
        return;
    }
    let i = ((y * W + x) * 3) as usize;
    px[i] = c[0];
    px[i + 1] = c[1];
    px[i + 2] = c[2];
}

fn rect(px: &mut [u8], x: u32, y: u32, w: u32, h: u32, c: [u8; 3]) {
    for dy in 0..h {
        for dx in 0..w {
            put(px, x + dx, y + dy, c);
        }
    }
}

fn hline(px: &mut [u8], x0: u32, x1: u32, y: u32, c: [u8; 3]) {
    let (a, b) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    for x in a..=b {
        put(px, x, y, c);
    }
}

fn vline(px: &mut [u8], x: u32, y0: u32, y1: u32, c: [u8; 3]) {
    let (a, b) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    for y in a..=b {
        put(px, x, y, c);
    }
}

fn line(px: &mut [u8], x0: u32, y0: u32, x1: u32, y1: u32, c: [u8; 3]) {
    let mut x0 = x0 as i32;
    let mut y0 = y0 as i32;
    let x1 = x1 as i32;
    let y1 = y1 as i32;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        put(px, x0 as u32, y0 as u32, c);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}
