use crate::error::{Error, Result};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavSettings {
    pub sample_rate: u32,
    pub bit_depth: u16,
}

impl WavSettings {
    pub fn new(sample_rate: u32, bit_depth: u16) -> Result<Self> {
        if bit_depth != 16 && bit_depth != 24 {
            return Err(Error::InvalidParam {
                message: format!("bit_depth must be 16 or 24, got {bit_depth}"),
            });
        }
        if !(8_000..=192_000).contains(&sample_rate) {
            return Err(Error::InvalidParam {
                message: format!("sample_rate must be 8000-192000, got {sample_rate}"),
            });
        }
        Ok(Self {
            sample_rate,
            bit_depth,
        })
    }

    pub fn spec(self) -> WavSpec {
        WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: self.bit_depth,
            sample_format: SampleFormat::Int,
        }
    }
}

/// Write mono PCM (16- or 24-bit integer).
pub fn write_wav(path: &Path, samples: &[f32], settings: WavSettings) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| Error::Io {
                path: Some(parent.to_path_buf()),
                source: e,
            })?;
        }
    }

    let spec = settings.spec();
    let mut writer = WavWriter::create(path, spec).map_err(|e| match e {
        hound::Error::IoError(io) => Error::Io {
            path: Some(path.to_path_buf()),
            source: io,
        },
        other => Error::Wav(other),
    })?;

    match settings.bit_depth {
        16 => {
            for &s in samples {
                let v = (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
                writer.write_sample(v)?;
            }
        }
        24 => {
            const MAX_24: f32 = 8_388_607.0;
            for &s in samples {
                let v = (s.clamp(-1.0, 1.0) * MAX_24) as i32;
                writer.write_sample(v)?;
            }
        }
        _ => unreachable!("validated in WavSettings::new"),
    }

    writer.finalize()?;
    Ok(())
}

/// Decoded mono PCM in -1..1, mixed down if the file has more than one channel.
#[derive(Clone, Debug)]
pub struct WavData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub channels: u16,
}

/// Read a WAV and mix to mono f32. Accepts 16/24/32-bit integer or 32-bit float.
pub fn read_wav(path: &Path) -> Result<WavData> {
    let mut reader = WavReader::open(path).map_err(|e| match e {
        hound::Error::IoError(io) => Error::Io {
            path: Some(path.to_path_buf()),
            source: io,
        },
        other => Error::Wav(other),
    })?;
    let spec = reader.spec();
    if !(8_000..=192_000).contains(&spec.sample_rate) {
        return Err(Error::InvalidParam {
            message: format!("sample_rate must be 8000-192000, got {}", spec.sample_rate),
        });
    }
    if spec.channels == 0 {
        return Err(Error::InvalidParam {
            message: "wav has zero channels".into(),
        });
    }

    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Int, 16) => {
            let raw = collect_samples::<i16>(&mut reader, path)?;
            mix_mono(&raw, spec.channels, |s| f32::from(s) / 32_768.0)
        }
        (SampleFormat::Int, 24) => {
            let raw = collect_samples::<i32>(&mut reader, path)?;
            mix_mono(&raw, spec.channels, |s| s as f32 / 8_388_608.0)
        }
        (SampleFormat::Int, 32) => {
            let raw = collect_samples::<i32>(&mut reader, path)?;
            mix_mono(&raw, spec.channels, |s| s as f32 / 2_147_483_648.0)
        }
        (SampleFormat::Float, 32) => {
            let raw = collect_samples::<f32>(&mut reader, path)?;
            mix_mono(&raw, spec.channels, |s| s)
        }
        (fmt, bits) => {
            return Err(Error::InvalidParam {
                message: format!("unsupported wav format {fmt:?} {bits}-bit"),
            });
        }
    };

    if samples.is_empty() {
        return Err(Error::InvalidParam {
            message: format!("wav is empty ({})", path.display()),
        });
    }

    Ok(WavData {
        samples,
        sample_rate: spec.sample_rate,
        bit_depth: spec.bits_per_sample,
        channels: spec.channels,
    })
}

fn collect_samples<S: hound::Sample>(
    reader: &mut WavReader<std::io::BufReader<std::fs::File>>,
    path: &Path,
) -> Result<Vec<S>> {
    reader
        .samples::<S>()
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| match e {
            hound::Error::IoError(io) => Error::Io {
                path: Some(path.to_path_buf()),
                source: io,
            },
            other => Error::Wav(other),
        })
}

fn mix_mono<T: Copy>(raw: &[T], channels: u16, to_f32: impl Fn(T) -> f32) -> Vec<f32> {
    let ch = usize::from(channels);
    if ch == 1 {
        return raw.iter().copied().map(to_f32).collect();
    }
    raw.chunks_exact(ch)
        .map(|frame| frame.iter().copied().map(&to_f32).sum::<f32>() / ch as f32)
        .collect()
}

/// Data-chunk payload size in bytes (not including headers).
pub fn pcm_data_bytes(n_samples: usize, bit_depth: u16, channels: u16) -> usize {
    n_samples * usize::from(channels) * (usize::from(bit_depth) / 8)
}
