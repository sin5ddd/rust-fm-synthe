use crate::error::{Error, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
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

/// Data-chunk payload size in bytes (not including headers).
pub fn pcm_data_bytes(n_samples: usize, bit_depth: u16, channels: u16) -> usize {
    n_samples * usize::from(channels) * (usize::from(bit_depth) / 8)
}
