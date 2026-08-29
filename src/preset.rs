use crate::algorithm::Algorithm;
use crate::error::{Error, Result};
use crate::operator::OperatorParams;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Factory bank, embedded so `cargo run` works without the presets/ directory.
const FACTORY: &[(&str, &str)] = &[
    ("sub-bass", include_str!("../presets/sub-bass.toml")),
    ("growl-bass", include_str!("../presets/growl-bass.toml")),
    ("metallic-hit", include_str!("../presets/metallic-hit.toml")),
    ("fm-riser", include_str!("../presets/fm-riser.toml")),
    ("stab-pluck", include_str!("../presets/stab-pluck.toml")),
    ("zap", include_str!("../presets/zap.toml")),
    ("glass-hit", include_str!("../presets/glass-hit.toml")),
];

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct PitchEnv {
    pub start_semitones: f64,
    pub end_semitones: f64,
    /// 0 = linear. Positive = slow start / fast end (ライザー向き).
    pub curve: f32,
}

impl Default for PitchEnv {
    fn default() -> Self {
        Self {
            start_semitones: 0.0,
            end_semitones: 0.0,
            curve: 0.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct LfoParams {
    pub rate_hz: f64,
    pub depth_cents: f64,
}

impl Default for LfoParams {
    fn default() -> Self {
        Self {
            rate_hz: 0.0,
            depth_cents: 0.0,
        }
    }
}

/// Scales modulation (not carrier audio) from note-on to end of the render.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ModSweep {
    pub start: f32,
    pub end: f32,
}

impl Default for ModSweep {
    fn default() -> Self {
        Self {
            start: 1.0,
            end: 1.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Preset {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub algorithm: Algorithm,
    #[serde(default)]
    pub feedback: f32,
    /// 1–4, typically 4.
    #[serde(default = "default_feedback_op")]
    pub feedback_op: u8,
    #[serde(default = "default_gain")]
    pub gain: f32,
    #[serde(default = "default_note")]
    pub default_note: u8,
    #[serde(default = "default_duration")]
    pub default_duration: f64,
    #[serde(default)]
    pub pitch: PitchEnv,
    #[serde(default)]
    pub lfo: LfoParams,
    #[serde(default)]
    pub mod_sweep: ModSweep,
    pub operators: Vec<OperatorParams>,
}

fn default_feedback_op() -> u8 {
    4
}
fn default_gain() -> f32 {
    1.0
}
fn default_note() -> u8 {
    48
}
fn default_duration() -> f64 {
    1.0
}

impl Preset {
    pub fn from_toml_str(source: &str, toml_text: &str) -> Result<Self> {
        let raw: Preset = toml::from_str(toml_text).map_err(|e| Error::PresetParse {
            source: source.to_string(),
            message: e.to_string(),
        })?;
        raw.validate(source)
    }

    fn validate(mut self, source: &str) -> Result<Self> {
        if self.operators.len() != 4 {
            return Err(Error::PresetParse {
                source: source.to_string(),
                message: format!("expected 4 operators, got {}", self.operators.len()),
            });
        }
        if !(1..=4).contains(&self.feedback_op) {
            return Err(Error::PresetParse {
                source: source.to_string(),
                message: format!("feedback_op must be 1-4, got {}", self.feedback_op),
            });
        }
        if self.name.trim().is_empty() {
            self.name = source.to_string();
        }
        Ok(self)
    }

    pub fn operator_array(&self) -> [OperatorParams; 4] {
        [
            self.operators[0].clone(),
            self.operators[1].clone(),
            self.operators[2].clone(),
            self.operators[3].clone(),
        ]
    }
}

#[derive(Clone, Debug)]
pub struct PresetInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_note: u8,
    pub default_duration: f64,
}

/// Identifiers in factory-bank order.
pub fn factory_ids() -> Vec<&'static str> {
    FACTORY.iter().map(|(id, _)| *id).collect()
}

pub fn factory_info() -> Result<Vec<PresetInfo>> {
    FACTORY
        .iter()
        .map(|(id, toml_text)| {
            let p = Preset::from_toml_str(id, toml_text)?;
            Ok(PresetInfo {
                id: (*id).to_string(),
                name: p.name,
                description: p.description,
                default_note: p.default_note,
                default_duration: p.default_duration,
            })
        })
        .collect()
}

pub fn load_factory(id: &str) -> Result<Preset> {
    let key = normalize_id(id);
    for (fid, toml_text) in FACTORY {
        if normalize_id(fid) == key {
            return Preset::from_toml_str(fid, toml_text);
        }
    }
    Err(Error::PresetNotFound {
        name: id.to_string(),
        searched: factory_ids().into_iter().map(str::to_string).collect(),
    })
}

pub fn load_preset_file(path: &Path) -> Result<Preset> {
    let text = fs::read_to_string(path).map_err(|e| Error::Io {
        path: Some(path.to_path_buf()),
        source: e,
    })?;
    Preset::from_toml_str(&path.display().to_string(), &text)
}

/// Resolve a preset name: exact factory id, `presets/<name>.toml` next to cwd,
/// or a path to a TOML file.
pub fn load_preset(name: &str) -> Result<Preset> {
    let as_path = Path::new(name);
    if as_path.is_file() {
        return load_preset_file(as_path);
    }

    match load_factory(name) {
        Ok(p) => return Ok(p),
        Err(Error::PresetNotFound { .. }) => {}
        Err(e) => return Err(e),
    }

    let mut searched = factory_ids()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let candidates = preset_file_candidates(name);
    for cand in &candidates {
        searched.push(cand.display().to_string());
        if cand.is_file() {
            return load_preset_file(cand);
        }
    }

    Err(Error::PresetNotFound {
        name: name.to_string(),
        searched,
    })
}

fn preset_file_candidates(name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let file = if name.ends_with(".toml") {
        PathBuf::from(name)
    } else {
        PathBuf::from(format!("{name}.toml"))
    };
    out.push(PathBuf::from("presets").join(&file));
    out.push(file);
    out
}

fn normalize_id(id: &str) -> String {
    id.trim().to_ascii_lowercase().replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_bank_parses() {
        for id in factory_ids() {
            load_factory(id).expect(id);
        }
    }

    #[test]
    fn unknown_preset_errors() {
        let err = load_factory("not-a-real-preset").unwrap_err();
        assert!(matches!(err, Error::PresetNotFound { .. }));
    }
}
