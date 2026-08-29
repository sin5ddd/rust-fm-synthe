use crate::algorithm::Algorithm;
use crate::error::{Error, Result};
use crate::filter::FilterParams;
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
    (
        "supersaw-bass",
        include_str!("../presets/supersaw-bass.toml"),
    ),
    ("filter-pluck", include_str!("../presets/filter-pluck.toml")),
    ("bp-growl", include_str!("../presets/bp-growl.toml")),
    ("hp-air", include_str!("../presets/hp-air.toml")),
    ("bd-808-boom", include_str!("../presets/bd-808-boom.toml")),
    ("bd-808-tight", include_str!("../presets/bd-808-tight.toml")),
    ("bd-909-punch", include_str!("../presets/bd-909-punch.toml")),
    (
        "bd-house-floor",
        include_str!("../presets/bd-house-floor.toml"),
    ),
    (
        "bd-techno-thud",
        include_str!("../presets/bd-techno-thud.toml"),
    ),
    ("bd-dnb-tight", include_str!("../presets/bd-dnb-tight.toml")),
    (
        "bd-neuro-growl",
        include_str!("../presets/bd-neuro-growl.toml"),
    ),
    (
        "bd-frenchcore",
        include_str!("../presets/bd-frenchcore.toml"),
    ),
    (
        "bd-gabber-stomp",
        include_str!("../presets/bd-gabber-stomp.toml"),
    ),
    ("bd-hardstyle", include_str!("../presets/bd-hardstyle.toml")),
    ("bd-lofi-dust", include_str!("../presets/bd-lofi-dust.toml")),
    ("bd-click", include_str!("../presets/bd-click.toml")),
    ("bd-sub", include_str!("../presets/bd-sub.toml")),
    (
        "bd-metal-ping",
        include_str!("../presets/bd-metal-ping.toml"),
    ),
    ("bd-cinematic", include_str!("../presets/bd-cinematic.toml")),
    (
        "bd-electro-zap",
        include_str!("../presets/bd-electro-zap.toml"),
    ),
    ("bd-808-dist", include_str!("../presets/bd-808-dist.toml")),
    ("bd-disco-dry", include_str!("../presets/bd-disco-dry.toml")),
    (
        "bd-jungle-round",
        include_str!("../presets/bd-jungle-round.toml"),
    ),
    ("bd-fm-noise", include_str!("../presets/bd-fm-noise.toml")),
    ("sd-808-snap", include_str!("../presets/sd-808-snap.toml")),
    (
        "sd-909-snappy",
        include_str!("../presets/sd-909-snappy.toml"),
    ),
    ("sd-pop-tight", include_str!("../presets/sd-pop-tight.toml")),
    (
        "sd-fat-backbeat",
        include_str!("../presets/sd-fat-backbeat.toml"),
    ),
    ("sd-rimshot", include_str!("../presets/sd-rimshot.toml")),
    (
        "sd-clap-snare",
        include_str!("../presets/sd-clap-snare.toml"),
    ),
    ("sd-gated-80s", include_str!("../presets/sd-gated-80s.toml")),
    (
        "sd-brush-dust",
        include_str!("../presets/sd-brush-dust.toml"),
    ),
    ("sd-piccolo", include_str!("../presets/sd-piccolo.toml")),
    ("sd-dnb-tight", include_str!("../presets/sd-dnb-tight.toml")),
    (
        "sd-jungle-round",
        include_str!("../presets/sd-jungle-round.toml"),
    ),
    (
        "sd-neuro-growl",
        include_str!("../presets/sd-neuro-growl.toml"),
    ),
    (
        "sd-frenchcore",
        include_str!("../presets/sd-frenchcore.toml"),
    ),
    (
        "sd-gabber-indust",
        include_str!("../presets/sd-gabber-indust.toml"),
    ),
    (
        "sd-trap-crisp",
        include_str!("../presets/sd-trap-crisp.toml"),
    ),
    (
        "sd-house-disco",
        include_str!("../presets/sd-house-disco.toml"),
    ),
    (
        "sd-metal-ping",
        include_str!("../presets/sd-metal-ping.toml"),
    ),
    (
        "sd-noise-layer",
        include_str!("../presets/sd-noise-layer.toml"),
    ),
    (
        "sd-tone-layer",
        include_str!("../presets/sd-tone-layer.toml"),
    ),
    ("sd-fm-long", include_str!("../presets/sd-fm-long.toml")),
    ("cp-house", include_str!("../presets/cp-house.toml")),
    (
        "lead-fm-pluck",
        include_str!("../presets/lead-fm-pluck.toml"),
    ),
    (
        "stab-fm-fifth",
        include_str!("../presets/stab-fm-fifth.toml"),
    ),
    ("reese-mid", include_str!("../presets/reese-mid.toml")),
    ("ld-fm-pluck", include_str!("../presets/ld-fm-pluck.toml")),
    (
        "ld-hollow-fifth",
        include_str!("../presets/ld-hollow-fifth.toml"),
    ),
    (
        "ld-house-pluck",
        include_str!("../presets/ld-house-pluck.toml"),
    ),
    ("ld-dnb-stab", include_str!("../presets/ld-dnb-stab.toml")),
    (
        "ld-supersaw-stab",
        include_str!("../presets/ld-supersaw-stab.toml"),
    ),
    ("ld-nylon", include_str!("../presets/ld-nylon.toml")),
    (
        "ld-bell-pluck",
        include_str!("../presets/ld-bell-pluck.toml"),
    ),
    ("ld-mallet", include_str!("../presets/ld-mallet.toml")),
    ("ld-perc", include_str!("../presets/ld-perc.toml")),
    ("ld-supersaw", include_str!("../presets/ld-supersaw.toml")),
    (
        "ld-unison-saw",
        include_str!("../presets/ld-unison-saw.toml"),
    ),
    (
        "ld-trance-gate",
        include_str!("../presets/ld-trance-gate.toml"),
    ),
    ("ld-hoover", include_str!("../presets/ld-hoover.toml")),
    ("ld-sync-fm", include_str!("../presets/ld-sync-fm.toml")),
    ("ld-formant", include_str!("../presets/ld-formant.toml")),
    ("ld-pulse", include_str!("../presets/ld-pulse.toml")),
    ("ld-anthem", include_str!("../presets/ld-anthem.toml")),
    ("ld-growl", include_str!("../presets/ld-growl.toml")),
    ("ld-metallic", include_str!("../presets/ld-metallic.toml")),
    (
        "ld-industrial",
        include_str!("../presets/ld-industrial.toml"),
    ),
    (
        "ld-frenchcore",
        include_str!("../presets/ld-frenchcore.toml"),
    ),
    ("ld-gabber", include_str!("../presets/ld-gabber.toml")),
    ("ld-acid", include_str!("../presets/ld-acid.toml")),
    (
        "ld-dist-pulse",
        include_str!("../presets/ld-dist-pulse.toml"),
    ),
    ("ld-sine", include_str!("../presets/ld-sine.toml")),
    ("ld-half-sine", include_str!("../presets/ld-half-sine.toml")),
    ("ld-choir", include_str!("../presets/ld-choir.toml")),
    ("ld-glass", include_str!("../presets/ld-glass.toml")),
    ("ld-music-box", include_str!("../presets/ld-music-box.toml")),
    ("ld-flute", include_str!("../presets/ld-flute.toml")),
    ("ld-organ", include_str!("../presets/ld-organ.toml")),
    ("ld-fifth-pad", include_str!("../presets/ld-fifth-pad.toml")),
    ("ld-octave", include_str!("../presets/ld-octave.toml")),
    ("ld-zap", include_str!("../presets/ld-zap.toml")),
    (
        "ld-drop-pluck",
        include_str!("../presets/ld-drop-pluck.toml"),
    ),
    ("ld-laser", include_str!("../presets/ld-laser.toml")),
    ("ld-vowel", include_str!("../presets/ld-vowel.toml")),
    ("ld-noisy-bp", include_str!("../presets/ld-noisy-bp.toml")),
    ("ld-reverse", include_str!("../presets/ld-reverse.toml")),
    ("ld-cinematic", include_str!("../presets/ld-cinematic.toml")),
    ("ld-crystal", include_str!("../presets/ld-crystal.toml")),
    ("ld-brass", include_str!("../presets/ld-brass.toml")),
    ("ld-reed", include_str!("../presets/ld-reed.toml")),
    ("ld-chip", include_str!("../presets/ld-chip.toml")),
    ("ld-wobble", include_str!("../presets/ld-wobble.toml")),
    ("ld-hardstyle", include_str!("../presets/ld-hardstyle.toml")),
    ("ld-arp-pluck", include_str!("../presets/ld-arp-pluck.toml")),
    ("ld-saw-pluck", include_str!("../presets/ld-saw-pluck.toml")),
    ("ld-ethereal", include_str!("../presets/ld-ethereal.toml")),
    ("ld-harpsi", include_str!("../presets/ld-harpsi.toml")),
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
    #[serde(default)]
    pub filter: FilterParams,
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

fn path_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "preset".into())
}

/// Filename stem for a default WAV (`dist/<id>.wav`).
///
/// Factory ids win over display names. A TOML path uses its file stem.
pub fn output_preset_id(preset_name: Option<&str>, preset_file: Option<&Path>) -> String {
    if let Some(path) = preset_file {
        return path_stem(path);
    }
    let Some(name) = preset_name else {
        return "preset".into();
    };
    let as_path = Path::new(name);
    if name.ends_with(".toml") || as_path.is_file() {
        return path_stem(as_path);
    }
    let key = normalize_id(name);
    for (fid, _) in FACTORY {
        if normalize_id(fid) == key {
            return (*fid).to_string();
        }
    }
    key
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

    #[test]
    fn output_id_uses_factory_id_not_toml_path() {
        assert_eq!(output_preset_id(Some("sub-bass"), None), "sub-bass");
        assert_eq!(output_preset_id(Some("SUB_BASS"), None), "sub-bass");
        assert_eq!(
            output_preset_id(None, Some(Path::new("presets/zap.toml"))),
            "zap"
        );
        assert_eq!(output_preset_id(Some("custom.toml"), None), "custom");
        assert_eq!(output_preset_id(Some("bd-808-boom"), None), "bd-808-boom");
        assert_eq!(output_preset_id(Some("sd-808-snap"), None), "sd-808-snap");
        assert_eq!(output_preset_id(Some("ld-fm-pluck"), None), "ld-fm-pluck");
    }

    #[test]
    fn factory_bd_kick_bank_has_twenty_ids() {
        let ids: Vec<_> = factory_ids()
            .into_iter()
            .filter(|id| id.starts_with("bd-"))
            .collect();
        assert_eq!(
            ids.len(),
            20,
            "expected exactly 20 bd-* factory kicks, got {}: {ids:?}",
            ids.len()
        );
        for id in &ids {
            load_factory(id).expect(id);
        }
    }

    #[test]
    fn factory_sd_snare_bank_has_twenty_ids() {
        let ids: Vec<_> = factory_ids()
            .into_iter()
            .filter(|id| id.starts_with("sd-"))
            .collect();
        assert_eq!(
            ids.len(),
            20,
            "expected exactly 20 sd-* factory snares, got {}: {ids:?}",
            ids.len()
        );
        for id in &ids {
            load_factory(id).expect(id);
        }
    }

    #[test]
    fn factory_ld_lead_bank_has_fifty_ids() {
        let ids: Vec<_> = factory_ids()
            .into_iter()
            .filter(|id| id.starts_with("ld-"))
            .collect();
        assert_eq!(
            ids.len(),
            50,
            "expected exactly 50 ld-* factory leads, got {}: {ids:?}",
            ids.len()
        );
        for id in &ids {
            load_factory(id).expect(id);
        }
    }

    #[test]
    fn factory_strudel_oneshots_parse() {
        const IDS: [&str; 4] = ["cp-house", "lead-fm-pluck", "stab-fm-fifth", "reese-mid"];
        for id in IDS {
            let p = load_factory(id).expect(id);
            assert_eq!(output_preset_id(Some(id), None), id);
            assert!(!p.operators.is_empty());
        }

        let clap = load_factory("cp-house").unwrap();
        assert!(
            (0.2..=0.35).contains(&clap.default_duration),
            "cp-house duration {}",
            clap.default_duration
        );

        for id in ["lead-fm-pluck", "stab-fm-fifth", "reese-mid"] {
            let p = load_factory(id).unwrap();
            assert_eq!(
                p.default_note, 48,
                "{id} pitch reference must be MIDI 48 (C3)"
            );
        }
        let lead = load_factory("lead-fm-pluck").unwrap();
        assert!(
            (lead.operators[0].ratio - 1.0).abs() < 1e-9,
            "lead carrier ratio must be 1 (C3), got {}",
            lead.operators[0].ratio
        );

        let stab = load_factory("stab-fm-fifth").unwrap();
        assert_eq!(stab.feedback, 0.0);
        let live: Vec<_> = stab.operators.iter().filter(|op| op.level > 1e-6).collect();
        assert_eq!(
            live.len(),
            2,
            "hollow fifth is two partials, got {}",
            live.len()
        );
        let mut ratios: Vec<f64> = live.iter().map(|op| op.ratio).collect();
        ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((ratios[0] - 1.0).abs() < 1e-9, "root ratio {}", ratios[0]);
        assert!((ratios[1] - 1.5).abs() < 1e-9, "fifth ratio {}", ratios[1]);
        for op in &live {
            assert_eq!(op.waveform, crate::operator::Waveform::Sine);
            assert!((op.detune_cents).abs() < 1e-9, "no detune on hollow fifth");
        }
        assert!(
            stab.operators
                .iter()
                .all(|op| (op.ratio - 1.25).abs() > 0.05),
            "no 5:4 / major-third ratio"
        );
    }
}
