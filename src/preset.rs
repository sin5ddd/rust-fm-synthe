use crate::algorithm::Algorithm;
use crate::error::{Error, Result};
use crate::filter::FilterParams;
use crate::operator::OperatorParams;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Factory bank, embedded so `cargo run` works without the presets/ directory.
/// Disk layout is `presets/<category>/<id>.toml`
/// (bass, bd, sd, ld, fx, perc, drone, pad-fresh, pad-sparkle).
macro_rules! factory_entry {
    ($dir:literal, $id:literal) => {
        (
            $id,
            include_str!(concat!("../presets/", $dir, "/", $id, ".toml")),
        )
    };
}

const FACTORY: &[(&str, &str)] = &[
    factory_entry!("bass", "sub-bass"),
    factory_entry!("bass", "growl-bass"),
    factory_entry!("perc", "metallic-hit"),
    factory_entry!("fx", "fm-riser"),
    factory_entry!("ld", "stab-pluck"),
    factory_entry!("fx", "zap"),
    factory_entry!("perc", "glass-hit"),
    factory_entry!("bass", "supersaw-bass"),
    factory_entry!("ld", "filter-pluck"),
    factory_entry!("bass", "bp-growl"),
    factory_entry!("fx", "hp-air"),
    factory_entry!("bd", "bd-808-boom"),
    factory_entry!("bd", "bd-808-tight"),
    factory_entry!("bd", "bd-909-punch"),
    factory_entry!("bd", "bd-house-floor"),
    factory_entry!("bd", "bd-techno-thud"),
    factory_entry!("bd", "bd-dnb-tight"),
    factory_entry!("bd", "bd-neuro-growl"),
    factory_entry!("bd", "bd-frenchcore"),
    factory_entry!("bd", "bd-gabber-stomp"),
    factory_entry!("bd", "bd-hardstyle"),
    factory_entry!("bd", "bd-lofi-dust"),
    factory_entry!("bd", "bd-click"),
    factory_entry!("bd", "bd-sub"),
    factory_entry!("bd", "bd-metal-ping"),
    factory_entry!("bd", "bd-cinematic"),
    factory_entry!("bd", "bd-electro-zap"),
    factory_entry!("bd", "bd-808-dist"),
    factory_entry!("bd", "bd-disco-dry"),
    factory_entry!("bd", "bd-jungle-round"),
    factory_entry!("bd", "bd-fm-noise"),
    factory_entry!("sd", "sd-808-snap"),
    factory_entry!("sd", "sd-909-snappy"),
    factory_entry!("sd", "sd-pop-tight"),
    factory_entry!("sd", "sd-fat-backbeat"),
    factory_entry!("sd", "sd-rimshot"),
    factory_entry!("sd", "sd-clap-snare"),
    factory_entry!("sd", "sd-gated-80s"),
    factory_entry!("sd", "sd-brush-dust"),
    factory_entry!("sd", "sd-piccolo"),
    factory_entry!("sd", "sd-dnb-tight"),
    factory_entry!("sd", "sd-jungle-round"),
    factory_entry!("sd", "sd-neuro-growl"),
    factory_entry!("sd", "sd-frenchcore"),
    factory_entry!("sd", "sd-gabber-indust"),
    factory_entry!("sd", "sd-trap-crisp"),
    factory_entry!("sd", "sd-house-disco"),
    factory_entry!("sd", "sd-metal-ping"),
    factory_entry!("sd", "sd-noise-layer"),
    factory_entry!("sd", "sd-tone-layer"),
    factory_entry!("sd", "sd-fm-long"),
    factory_entry!("perc", "cp-house"),
    factory_entry!("ld", "lead-fm-pluck"),
    factory_entry!("ld", "stab-fm-fifth"),
    factory_entry!("ld", "stab-fm-major"),
    factory_entry!("bass", "reese-mid"),
    factory_entry!("ld", "ld-fm-pluck"),
    factory_entry!("ld", "ld-hollow-fifth"),
    factory_entry!("ld", "ld-house-pluck"),
    factory_entry!("ld", "ld-dnb-stab"),
    factory_entry!("ld", "ld-supersaw-stab"),
    factory_entry!("ld", "ld-nylon"),
    factory_entry!("ld", "ld-bell-pluck"),
    factory_entry!("ld", "ld-mallet"),
    factory_entry!("ld", "ld-perc"),
    factory_entry!("ld", "ld-supersaw"),
    factory_entry!("ld", "ld-unison-saw"),
    factory_entry!("ld", "ld-trance-gate"),
    factory_entry!("ld", "ld-hoover"),
    factory_entry!("ld", "ld-sync-fm"),
    factory_entry!("ld", "ld-formant"),
    factory_entry!("ld", "ld-pulse"),
    factory_entry!("ld", "ld-anthem"),
    factory_entry!("ld", "ld-growl"),
    factory_entry!("ld", "ld-metallic"),
    factory_entry!("ld", "ld-industrial"),
    factory_entry!("ld", "ld-frenchcore"),
    factory_entry!("ld", "ld-gabber"),
    factory_entry!("ld", "ld-acid"),
    factory_entry!("ld", "ld-dist-pulse"),
    factory_entry!("ld", "ld-sine"),
    factory_entry!("ld", "ld-half-sine"),
    factory_entry!("ld", "ld-choir"),
    factory_entry!("ld", "ld-glass"),
    factory_entry!("ld", "ld-music-box"),
    factory_entry!("ld", "ld-flute"),
    factory_entry!("ld", "ld-organ"),
    factory_entry!("ld", "ld-fifth-pad"),
    factory_entry!("ld", "ld-octave"),
    factory_entry!("ld", "ld-zap"),
    factory_entry!("ld", "ld-drop-pluck"),
    factory_entry!("ld", "ld-laser"),
    factory_entry!("ld", "ld-vowel"),
    factory_entry!("ld", "ld-noisy-bp"),
    factory_entry!("ld", "ld-reverse"),
    factory_entry!("ld", "ld-cinematic"),
    factory_entry!("ld", "ld-crystal"),
    factory_entry!("ld", "ld-brass"),
    factory_entry!("ld", "ld-reed"),
    factory_entry!("ld", "ld-chip"),
    factory_entry!("ld", "ld-wobble"),
    factory_entry!("ld", "ld-hardstyle"),
    factory_entry!("ld", "ld-arp-pluck"),
    factory_entry!("ld", "ld-saw-pluck"),
    factory_entry!("ld", "ld-ethereal"),
    factory_entry!("ld", "ld-harpsi"),
    factory_entry!("fx", "fx-rev-cym"),
    factory_entry!("fx", "fx-rev-crash"),
    factory_entry!("fx", "fx-rev-hat"),
    factory_entry!("fx", "fx-rev-cym-long"),
    factory_entry!("fx", "fx-rev-cym-bright"),
    factory_entry!("fx", "fx-rev-cym-dark"),
    factory_entry!("fx", "fx-rev-crash-metal"),
    factory_entry!("fx", "fx-rev-air"),
    factory_entry!("fx", "fx-rev-cym-noise"),
    factory_entry!("fx", "fx-rev-splash"),
    factory_entry!("fx", "fx-noise-hit"),
    factory_entry!("fx", "fx-noise-burst"),
    factory_entry!("fx", "fx-metal-crash"),
    factory_entry!("fx", "fx-glass-smash"),
    factory_entry!("fx", "fx-impact"),
    factory_entry!("fx", "fx-boom"),
    factory_entry!("fx", "fx-sub-drop"),
    factory_entry!("fx", "fx-impact-mid"),
    factory_entry!("fx", "fx-uplifter"),
    factory_entry!("fx", "fx-riser-noise"),
    factory_entry!("fx", "fx-riser-pitch"),
    factory_entry!("fx", "fx-riser-saw"),
    factory_entry!("fx", "fx-downlifter"),
    factory_entry!("fx", "fx-fall"),
    factory_entry!("fx", "fx-downlifter-noise"),
    factory_entry!("fx", "fx-whoosh"),
    factory_entry!("fx", "fx-wind"),
    factory_entry!("fx", "fx-passby"),
    factory_entry!("fx", "fx-laser"),
    factory_entry!("fx", "fx-zap"),
    factory_entry!("fx", "fx-blip"),
    factory_entry!("fx", "fx-laser-fall"),
    factory_entry!("fx", "fx-sweep-bp"),
    factory_entry!("fx", "fx-formant-ah"),
    factory_entry!("fx", "fx-formant-oh"),
    factory_entry!("fx", "fx-tape-stop"),
    factory_entry!("fx", "fx-rev-verb"),
    factory_entry!("fx", "fx-clang"),
    factory_entry!("fx", "fx-frenchcore-ns"),
    factory_entry!("fx", "fx-gabber-stab"),
    factory_entry!("fx", "fx-crackle"),
    factory_entry!("fx", "fx-radio-stab"),
    factory_entry!("fx", "fx-alarm"),
    factory_entry!("fx", "fx-siren"),
    factory_entry!("fx", "fx-down-to-kick"),
    factory_entry!("fx", "fx-trans-fill"),
    factory_entry!("fx", "fx-impact-dnb"),
    factory_entry!("fx", "fx-whoosh-hp"),
    factory_entry!("fx", "fx-riser-filter"),
    factory_entry!("fx", "fx-hoover-fall"),
    factory_entry!("bass", "bs-808-sub"),
    factory_entry!("bass", "bs-reese-dark"),
    factory_entry!("bass", "bs-reese-bright"),
    factory_entry!("bass", "bs-reese-neuro"),
    factory_entry!("bass", "bs-wobble"),
    factory_entry!("bass", "bs-acid"),
    factory_entry!("bass", "bs-frenchcore"),
    factory_entry!("bass", "bs-gabber"),
    factory_entry!("bass", "bs-hoover"),
    factory_entry!("bass", "bs-dist-square"),
    factory_entry!("bass", "bs-house-tight"),
    factory_entry!("bass", "bs-amen-sub"),
    factory_entry!("bass", "bs-growl-2"),
    factory_entry!("bass", "bs-sine-sub"),
    factory_entry!("bass", "bs-metal-fm"),
    factory_entry!("perc", "pc-hat-closed"),
    factory_entry!("perc", "pc-hat-open"),
    factory_entry!("perc", "pc-hat-house"),
    factory_entry!("perc", "pc-hat-dnb-cl"),
    factory_entry!("perc", "pc-hat-dnb-op"),
    factory_entry!("perc", "pc-hat-fc"),
    factory_entry!("perc", "pc-hat-pedal"),
    factory_entry!("perc", "pc-hat-tight"),
    factory_entry!("perc", "pc-hat-dark"),
    factory_entry!("perc", "pc-hat-noise"),
    factory_entry!("perc", "pc-hat-chip"),
    factory_entry!("perc", "pc-hat-fc-op"),
    factory_entry!("perc", "pc-shaker"),
    factory_entry!("perc", "pc-shaker-short"),
    factory_entry!("perc", "pc-tamb"),
    factory_entry!("perc", "pc-tamb-roll"),
    factory_entry!("perc", "pc-cabasa"),
    factory_entry!("perc", "pc-conga-hi"),
    factory_entry!("perc", "pc-conga-lo"),
    factory_entry!("perc", "pc-bongo-hi"),
    factory_entry!("perc", "pc-bongo-lo"),
    factory_entry!("perc", "pc-tom-hi"),
    factory_entry!("perc", "pc-tom-mid"),
    factory_entry!("perc", "pc-tom-lo"),
    factory_entry!("perc", "pc-rim"),
    factory_entry!("perc", "pc-cowbell"),
    factory_entry!("perc", "pc-clave"),
    factory_entry!("perc", "pc-snap"),
    factory_entry!("perc", "pc-snaps"),
    factory_entry!("perc", "pc-snap-lo"),
    factory_entry!("perc", "pc-triangle"),
    factory_entry!("perc", "pc-ride-fm"),
    factory_entry!("perc", "pc-ride-bell"),
    factory_entry!("perc", "pc-woodblock"),
    factory_entry!("perc", "pc-agogo-hi"),
    factory_entry!("perc", "pc-agogo-lo"),
    factory_entry!("perc", "pc-tick-metal"),
    factory_entry!("perc", "pc-tick-indust"),
    factory_entry!("perc", "pc-tick-clock"),
    factory_entry!("perc", "pc-clap-dry"),
    factory_entry!("perc", "pc-clap-room"),
    factory_entry!("perc", "pc-clap-gate"),
    factory_entry!("perc", "pc-zap"),
    factory_entry!("perc", "pc-zap-lo"),
    factory_entry!("perc", "pc-foley-click"),
    factory_entry!("perc", "pc-foley-thud"),
    factory_entry!("perc", "pc-foley-scratch"),
    factory_entry!("perc", "pc-chime"),
    factory_entry!("perc", "pc-guiro"),
    factory_entry!("perc", "pc-stick"),
    factory_entry!("drone", "dr-sine-sub"),
    factory_entry!("drone", "dr-sub-octave"),
    factory_entry!("drone", "dr-rumble"),
    factory_entry!("drone", "dr-reese-dark"),
    factory_entry!("drone", "dr-reese-wide"),
    factory_entry!("drone", "dr-supersaw-low"),
    factory_entry!("drone", "dr-pulse-rumble"),
    factory_entry!("drone", "dr-fifth-hollow"),
    factory_entry!("drone", "dr-octave-stack"),
    factory_entry!("drone", "dr-minor-dark"),
    factory_entry!("drone", "dr-fm-evolve"),
    factory_entry!("drone", "dr-fm-index"),
    factory_entry!("drone", "dr-noisy-bp"),
    factory_entry!("drone", "dr-metal-distant"),
    factory_entry!("drone", "dr-choir-low"),
    factory_entry!("drone", "dr-choir-dark"),
    factory_entry!("drone", "dr-trailer-bloom"),
    factory_entry!("drone", "dr-impact-hold"),
    factory_entry!("drone", "dr-reverse-hold"),
    factory_entry!("drone", "dr-riser-slow"),
    factory_entry!("drone", "dr-underwater"),
    factory_entry!("drone", "dr-industrial"),
    factory_entry!("drone", "dr-scifi-hum"),
    factory_entry!("drone", "dr-horror"),
    factory_entry!("drone", "dr-ambient-dark"),
    factory_entry!("drone", "dr-brass-pad"),
    factory_entry!("drone", "dr-brass-distant"),
    factory_entry!("drone", "dr-thunder-bed"),
    factory_entry!("drone", "dr-dystopia"),
    factory_entry!("drone", "dr-pad-dark"),
    factory_entry!("drone", "dr-hum-grid"),
    factory_entry!("drone", "dr-void"),
    factory_entry!("drone", "dr-abyss"),
    factory_entry!("drone", "dr-cathedral"),
    factory_entry!("drone", "dr-engine"),
    factory_entry!("drone", "dr-reactor"),
    factory_entry!("drone", "dr-ice-cave"),
    factory_entry!("drone", "dr-fog"),
    factory_entry!("drone", "dr-warfare"),
    factory_entry!("drone", "dr-ritual"),
    factory_entry!("drone", "dr-ghost-choir"),
    factory_entry!("drone", "dr-metal-bed"),
    factory_entry!("drone", "dr-pulse-fifth"),
    factory_entry!("drone", "dr-saw-minor"),
    factory_entry!("drone", "dr-fm-bell-low"),
    factory_entry!("drone", "dr-wobble-slow"),
    factory_entry!("drone", "dr-formant-low"),
    factory_entry!("drone", "dr-tape-hum"),
    factory_entry!("drone", "dr-storm"),
    factory_entry!("drone", "dr-score-hold"),
    factory_entry!("pad-fresh", "pf-morning"),
    factory_entry!("pad-fresh", "pf-juno-air"),
    factory_entry!("pad-fresh", "pf-chorus-wide"),
    factory_entry!("pad-fresh", "pf-flute-pad"),
    factory_entry!("pad-fresh", "pf-choir-air"),
    factory_entry!("pad-fresh", "pf-fifth-open"),
    factory_entry!("pad-fresh", "pf-ninth-open"),
    factory_entry!("pad-fresh", "pf-lydian-sky"),
    factory_entry!("pad-fresh", "pf-major-soft"),
    factory_entry!("pad-fresh", "pf-glass-air"),
    factory_entry!("pad-fresh", "pf-dawn"),
    factory_entry!("pad-fresh", "pf-breeze"),
    factory_entry!("pad-fresh", "pf-sky-open"),
    factory_entry!("pad-fresh", "pf-cloud"),
    factory_entry!("pad-fresh", "pf-horizon"),
    factory_entry!("pad-fresh", "pf-meadow"),
    factory_entry!("pad-fresh", "pf-clear-saw"),
    factory_entry!("pad-fresh", "pf-pulse-air"),
    factory_entry!("pad-fresh", "pf-octave-light"),
    factory_entry!("pad-fresh", "pf-silk"),
    factory_entry!("pad-fresh", "pf-ivory"),
    factory_entry!("pad-fresh", "pf-harp-air"),
    factory_entry!("pad-fresh", "pf-organ-light"),
    factory_entry!("pad-fresh", "pf-reed-soft"),
    factory_entry!("pad-fresh", "pf-water-air"),
    factory_entry!("pad-fresh", "pf-alpine"),
    factory_entry!("pad-fresh", "pf-spring"),
    factory_entry!("pad-fresh", "pf-linen"),
    factory_entry!("pad-fresh", "pf-halo"),
    factory_entry!("pad-fresh", "pf-wide-major"),
    factory_entry!("pad-sparkle", "ps-crystal"),
    factory_entry!("pad-sparkle", "ps-bell-hold"),
    factory_entry!("pad-sparkle", "ps-shimmer"),
    factory_entry!("pad-sparkle", "ps-music-box"),
    factory_entry!("pad-sparkle", "ps-ice-shine"),
    factory_entry!("pad-sparkle", "ps-starlight"),
    factory_entry!("pad-sparkle", "ps-glitter"),
    factory_entry!("pad-sparkle", "ps-chime-pad"),
    factory_entry!("pad-sparkle", "ps-fm-sparkle"),
    factory_entry!("pad-sparkle", "ps-chorus-shine"),
    factory_entry!("pad-sparkle", "ps-glass-bell"),
    factory_entry!("pad-sparkle", "ps-celesta"),
    factory_entry!("pad-sparkle", "ps-prism"),
    factory_entry!("pad-sparkle", "ps-frost"),
    factory_entry!("pad-sparkle", "ps-twinkle"),
    factory_entry!("pad-sparkle", "ps-aurora"),
    factory_entry!("pad-sparkle", "ps-diamond"),
    factory_entry!("pad-sparkle", "ps-silver"),
    factory_entry!("pad-sparkle", "ps-glisten"),
    factory_entry!("pad-sparkle", "ps-high-partials"),
    factory_entry!("pad-sparkle", "ps-inharmonic"),
    factory_entry!("pad-sparkle", "ps-celestial"),
    factory_entry!("pad-sparkle", "ps-spark-evolve"),
    factory_entry!("pad-sparkle", "ps-halo-shine"),
    factory_entry!("pad-sparkle", "ps-crystal-choir"),
    factory_entry!("pad-sparkle", "ps-bell-air"),
    factory_entry!("pad-sparkle", "ps-glock-pad"),
    factory_entry!("pad-sparkle", "ps-shine-fifth"),
    factory_entry!("pad-sparkle", "ps-ice-choir"),
    factory_entry!("pad-sparkle", "ps-quartz"),
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

/// Resolve a preset name: exact factory id, `presets/<category>/<name>.toml`
/// (or a nested path under `presets/`), or a path to a TOML file.
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
    push_unique(&mut out, PathBuf::from("presets").join(&file));
    push_unique(&mut out, file.clone());

    if let Some(want) = file.file_name() {
        let mut found = Vec::new();
        collect_toml_files(Path::new("presets"), &mut found);
        for path in found {
            if path.file_name() == Some(want) {
                push_unique(&mut out, path);
            }
        }
    }
    out
}

fn push_unique(out: &mut Vec<PathBuf>, path: PathBuf) {
    if !out.iter().any(|p| p == &path) {
        out.push(path);
    }
}

fn collect_toml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            out.push(path);
        }
    }
    dirs.sort();
    for nested in dirs {
        collect_toml_files(&nested, out);
    }
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
            output_preset_id(None, Some(Path::new("presets/fx/zap.toml"))),
            "zap"
        );
        assert_eq!(output_preset_id(Some("custom.toml"), None), "custom");
        assert_eq!(output_preset_id(Some("bd-808-boom"), None), "bd-808-boom");
        assert_eq!(output_preset_id(Some("sd-808-snap"), None), "sd-808-snap");
        assert_eq!(output_preset_id(Some("ld-fm-pluck"), None), "ld-fm-pluck");
        assert_eq!(output_preset_id(Some("fx-rev-cym"), None), "fx-rev-cym");
        assert_eq!(output_preset_id(Some("bs-808-sub"), None), "bs-808-sub");
        assert_eq!(output_preset_id(Some("pc-hat-closed"), None), "pc-hat-closed");
        assert_eq!(output_preset_id(Some("dr-sine-sub"), None), "dr-sine-sub");
        assert_eq!(output_preset_id(Some("pf-morning"), None), "pf-morning");
        assert_eq!(output_preset_id(Some("ps-crystal"), None), "ps-crystal");
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
    fn factory_fx_bank_has_fifty_ids() {
        let ids: Vec<_> = factory_ids()
            .into_iter()
            .filter(|id| id.starts_with("fx-"))
            .collect();
        assert_eq!(
            ids.len(),
            50,
            "expected exactly 50 fx-* factory FX, got {}: {ids:?}",
            ids.len()
        );
        for id in &ids {
            load_factory(id).expect(id);
        }
    }

    #[test]
    fn factory_bs_bass_bank_has_fifteen_ids() {
        let ids: Vec<_> = factory_ids()
            .into_iter()
            .filter(|id| id.starts_with("bs-"))
            .collect();
        assert_eq!(
            ids.len(),
            15,
            "expected exactly 15 bs-* factory basses, got {}: {ids:?}",
            ids.len()
        );
        for id in &ids {
            load_factory(id).expect(id);
        }
    }

    #[test]
    fn factory_pc_perc_bank_has_fifty_ids() {
        let ids: Vec<_> = factory_ids()
            .into_iter()
            .filter(|id| id.starts_with("pc-"))
            .collect();
        assert_eq!(
            ids.len(),
            50,
            "expected exactly 50 pc-* factory perc, got {}: {ids:?}",
            ids.len()
        );
        for id in &ids {
            load_factory(id).expect(id);
        }
    }

    #[test]
    fn factory_dr_drone_bank_has_fifty_ids() {
        let ids: Vec<_> = factory_ids()
            .into_iter()
            .filter(|id| id.starts_with("dr-"))
            .collect();
        assert_eq!(
            ids.len(),
            50,
            "expected exactly 50 dr-* factory drones, got {}: {ids:?}",
            ids.len()
        );
        for id in &ids {
            let p = load_factory(id).expect(id);
            assert!(
                (16.2..=18.0).contains(&p.default_duration),
                "{id} default_duration {} must be ~16 s+ (8 bars @ 120 BPM)",
                p.default_duration
            );
        }
    }

    #[test]
    fn factory_pf_pad_fresh_bank_has_thirty_ids() {
        let ids: Vec<_> = factory_ids()
            .into_iter()
            .filter(|id| id.starts_with("pf-"))
            .collect();
        assert_eq!(
            ids.len(),
            30,
            "expected exactly 30 pf-* factory pads, got {}: {ids:?}",
            ids.len()
        );
        for id in &ids {
            let p = load_factory(id).expect(id);
            assert!(
                (16.2..=18.0).contains(&p.default_duration),
                "{id} default_duration {} must be ~16 s+ (8 bars @ 120 BPM)",
                p.default_duration
            );
            assert!(
                (48..=72).contains(&p.default_note),
                "{id} default_note {} must be MIDI 48–72 (C3–C5)",
                p.default_note
            );
        }
    }

    #[test]
    fn factory_ps_pad_sparkle_bank_has_thirty_ids() {
        let ids: Vec<_> = factory_ids()
            .into_iter()
            .filter(|id| id.starts_with("ps-"))
            .collect();
        assert_eq!(
            ids.len(),
            30,
            "expected exactly 30 ps-* factory pads, got {}: {ids:?}",
            ids.len()
        );
        for id in &ids {
            let p = load_factory(id).expect(id);
            assert!(
                (16.2..=18.0).contains(&p.default_duration),
                "{id} default_duration {} must be ~16 s+ (8 bars @ 120 BPM)",
                p.default_duration
            );
            assert!(
                (48..=72).contains(&p.default_note),
                "{id} default_note {} must be MIDI 48–72 (C3–C5)",
                p.default_note
            );
        }
    }

    #[test]
    fn load_preset_file_from_category_folder() {
        let p =
            load_preset_file(Path::new("presets/bass/sub-bass.toml")).expect("nested factory toml");
        assert!(!p.name.is_empty());
        let nested = load_preset("presets/bd/bd-808-boom.toml").expect("nested path");
        assert!(!nested.operators.is_empty());
    }

    #[test]
    fn factory_toml_files_live_in_category_folders() {
        for id in factory_ids() {
            let matches: Vec<_> = preset_file_candidates(id)
                .into_iter()
                .filter(|p| p.is_file())
                .collect();
            assert!(
                !matches.is_empty(),
                "factory `{id}` has no presets/<category>/{id}.toml on disk"
            );
            for path in &matches {
                let parent = path.parent().and_then(|p| p.file_name());
                assert_ne!(
                    parent.map(|s| s.to_string_lossy().into_owned()).as_deref(),
                    Some("presets"),
                    "`{id}` still lives at presets root: {}",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn factory_strudel_oneshots_parse() {
        const IDS: [&str; 5] = [
            "cp-house",
            "lead-fm-pluck",
            "stab-fm-fifth",
            "stab-fm-major",
            "reese-mid",
        ];
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

        for id in [
            "lead-fm-pluck",
            "stab-fm-fifth",
            "stab-fm-major",
            "reese-mid",
        ] {
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

        let major = load_factory("stab-fm-major").unwrap();
        assert_eq!(major.feedback, 0.0);
        assert!(
            (8.0..=8.5).contains(&major.default_duration),
            "stab-fm-major duration {} (held 4 bars @ 120 BPM)",
            major.default_duration
        );
        let live: Vec<_> = major
            .operators
            .iter()
            .filter(|op| op.level > 1e-6)
            .collect();
        assert_eq!(
            live.len(),
            3,
            "major triad is three partials, got {}",
            live.len()
        );
        let mut ratios: Vec<f64> = live.iter().map(|op| op.ratio).collect();
        ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((ratios[0] - 1.0).abs() < 1e-9, "root ratio {}", ratios[0]);
        assert!(
            (ratios[1] - 1.25).abs() < 1e-9,
            "major-third ratio {}",
            ratios[1]
        );
        assert!((ratios[2] - 1.5).abs() < 1e-9, "fifth ratio {}", ratios[2]);
        for op in &live {
            assert_eq!(op.waveform, crate::operator::Waveform::Sine);
            assert!((op.detune_cents).abs() < 1e-9, "no detune on major triad");
        }
        assert!(
            major
                .operators
                .iter()
                .all(|op| op.level < 1e-6 || (op.ratio - 1.2).abs() > 0.02),
            "no 6:5 / minor-third ratio"
        );
    }
}
