use clap::{Parser, Subcommand};
use fm_synth::{
    analyze_all_factory, analyze_buffer, analyze_preset, default_png_path, default_wav_path,
    factory_info, load_preset, load_preset_file, output_preset_id, read_wav, render_all_factory,
    render_preset_wav, write_analysis_bundle, write_wav, Algorithm, Analysis, AnalyzeOpts,
    AnalyzeWriteReport, ExportParams, LayerMode, Result as SynthResult, WavRenderReport,
    WavSettings, DEFAULT_OUTPUT_DIR,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "fm-synth",
    version,
    about = "FMシンセ: EDM / DnB 向けサンプルとFXをWAVに書き出す",
    long_about = "4オペFMエンジンのオフラインレンダラ。プリセットを指定してモノラルWAVを書き出す。\n再生デバイスは使わない。書き出したWAVをサンプラーやDAWに貼る用途。",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 工場プリセットの一覧
    List,
    /// アルゴリズム 1–8 の配線を表示
    Algos,
    /// プリセットをレンダリングして WAV に書き出す
    Render {
        /// 工場プリセット名、または TOML へのパス
        #[arg(short, long)]
        preset: Option<String>,
        /// プリセットTOMLを直接指定（`--preset` より優先）
        #[arg(long)]
        preset_file: Option<PathBuf>,
        /// 出力WAVパス。省略時は `dist/<preset-id>.wav`
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// MIDIノート (0–127)。未指定ならプリセットの default_note
        #[arg(long)]
        note: Option<u8>,
        /// 周波数Hz。指定時は `--note` より優先
        #[arg(long)]
        hz: Option<f64>,
        /// 秒。未指定ならプリセットの default_duration
        #[arg(short, long)]
        duration: Option<f64>,
        /// ベロシティ 0.0–1.0
        #[arg(long, default_value_t = 0.9)]
        velocity: f32,
        /// サンプルレート
        #[arg(long, default_value_t = 44_100)]
        sample_rate: u32,
        /// ビット深度 16 または 24
        #[arg(long, default_value_t = 16)]
        bit_depth: u16,
        /// 同じ4OPパッチを別音程でもう一度レンダして混ぜる。
        /// `auto`=工場リードはオクターブ上（薄いとき5度）。工場FXのピッチ系は
        /// −12/−24（薄いとき−36）。他の工場FXは−12。`none` / `octave` /
        /// `octave-down` / `octave-down,octave-down-2` / `0,-12,-24`。
        /// `[fx.chorus] intervals` のピッチシフトではない。
        #[arg(long, default_value = "auto")]
        layers: String,
    },
    /// 工場バンクの全プリセットを WAV に書き出す（既定: dist/<id>.wav）
    RenderAll {
        /// 出力ディレクトリ。省略時は `dist/`
        #[arg(short = 'o', long = "output-dir", default_value = DEFAULT_OUTPUT_DIR)]
        output_dir: PathBuf,
        /// MIDIノート (0–127)。未指定なら各プリセットの default_note
        #[arg(long)]
        note: Option<u8>,
        /// 周波数Hz。指定時は `--note` より優先（全プリセット共通）
        #[arg(long)]
        hz: Option<f64>,
        /// 秒。未指定なら各プリセットの default_duration
        #[arg(short, long)]
        duration: Option<f64>,
        /// ベロシティ 0.0–1.0
        #[arg(long, default_value_t = 0.9)]
        velocity: f32,
        /// サンプルレート
        #[arg(long, default_value_t = 44_100)]
        sample_rate: u32,
        /// ビット深度 16 または 24
        #[arg(long, default_value_t = 16)]
        bit_depth: u16,
        /// 同じ4OPパッチを別音程でもう一度レンダして混ぜる。
        /// `auto`=工場リードはオクターブ上（薄いとき5度）。工場FXのピッチ系は
        /// −12/−24（薄いとき−36）。他の工場FXは−12。`none` / `octave` /
        /// `octave-down` / `octave-down,octave-down-2` / `0,-12,-24`。
        /// `[fx.chorus] intervals` のピッチシフトではない。
        #[arg(long, default_value = "auto")]
        layers: String,
    },
    /// WAV またはプリセットを分析し、スペクトログラム PNG と JSON を書く
    Analyze {
        /// 既存 WAV
        #[arg(long)]
        wav: Option<PathBuf>,
        /// 工場プリセット名
        #[arg(short, long)]
        preset: Option<String>,
        /// プリセットTOMLを直接指定（`--preset` より優先）
        #[arg(long)]
        preset_file: Option<PathBuf>,
        /// 出力 PNG パス。省略時は WAV と同じ stem、または `dist/<id>.png`
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// 照合用の意図テキスト（JSON に載せる）
        #[arg(long)]
        intent: Option<String>,
        /// MIDIノート (0–127)。未指定ならプリセットの default_note
        #[arg(long)]
        note: Option<u8>,
        /// 周波数Hz。指定時は `--note` より優先
        #[arg(long)]
        hz: Option<f64>,
        /// 秒。未指定ならプリセットの default_duration
        #[arg(short, long)]
        duration: Option<f64>,
        /// ベロシティ 0.0–1.0
        #[arg(long, default_value_t = 0.9)]
        velocity: f32,
        /// サンプルレート（プリセットをレンダするとき）
        #[arg(long, default_value_t = 44_100)]
        sample_rate: u32,
        /// ビット深度 16 または 24（プリセットをレンダするとき）
        #[arg(long, default_value_t = 16)]
        bit_depth: u16,
        /// 同じ4OPパッチを別音程でもう一度レンダして混ぜる。
        /// `auto`=工場リードはオクターブ上（薄いとき5度）。工場FXのピッチ系は
        /// −12/−24（薄いとき−36）。他の工場FXは−12。`none` / `octave` /
        /// `octave-down` / `octave-down,octave-down-2` / `0,-12,-24`。
        /// `[fx.chorus] intervals` のピッチシフトではない。
        #[arg(long, default_value = "auto")]
        layers: String,
    },
    /// 工場バンクを分析して PNG / JSON を書く（既定: dist/<id>.png）
    AnalyzeAll {
        /// 出力ディレクトリ。省略時は `dist/`
        #[arg(short = 'o', long = "output-dir", default_value = DEFAULT_OUTPUT_DIR)]
        output_dir: PathBuf,
        /// 照合用の意図テキスト（各 JSON に載せる）
        #[arg(long)]
        intent: Option<String>,
        /// MIDIノート (0–127)。未指定なら各プリセットの default_note
        #[arg(long)]
        note: Option<u8>,
        /// 周波数Hz。指定時は `--note` より優先（全プリセット共通）
        #[arg(long)]
        hz: Option<f64>,
        /// 秒。未指定なら各プリセットの default_duration
        #[arg(short, long)]
        duration: Option<f64>,
        /// ベロシティ 0.0–1.0
        #[arg(long, default_value_t = 0.9)]
        velocity: f32,
        /// サンプルレート
        #[arg(long, default_value_t = 44_100)]
        sample_rate: u32,
        /// ビット深度 16 または 24
        #[arg(long, default_value_t = 16)]
        bit_depth: u16,
        /// 同じ4OPパッチを別音程でもう一度レンダして混ぜる。
        /// `auto`=工場リードはオクターブ上（薄いとき5度）。工場FXのピッチ系は
        /// −12/−24（薄いとき−36）。他の工場FXは−12。`none` / `octave` /
        /// `octave-down` / `octave-down,octave-down-2` / `0,-12,-24`。
        /// `[fx.chorus] intervals` のピッチシフトではない。
        #[arg(long, default_value = "auto")]
        layers: String,
    },
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> SynthResult<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::List => cmd_list(),
        Command::Algos => {
            cmd_algos();
            Ok(())
        }
        Command::Render {
            preset,
            preset_file,
            output,
            note,
            hz,
            duration,
            velocity,
            sample_rate,
            bit_depth,
            layers,
        } => cmd_render(
            preset,
            preset_file,
            output,
            export_params(
                note,
                hz,
                duration,
                velocity,
                sample_rate,
                bit_depth,
                &layers,
            )?,
        ),
        Command::RenderAll {
            output_dir,
            note,
            hz,
            duration,
            velocity,
            sample_rate,
            bit_depth,
            layers,
        } => cmd_render_all(
            output_dir,
            export_params(
                note,
                hz,
                duration,
                velocity,
                sample_rate,
                bit_depth,
                &layers,
            )?,
        ),
        Command::Analyze {
            wav,
            preset,
            preset_file,
            output,
            intent,
            note,
            hz,
            duration,
            velocity,
            sample_rate,
            bit_depth,
            layers,
        } => cmd_analyze(
            wav,
            preset,
            preset_file,
            output,
            intent,
            export_params(
                note,
                hz,
                duration,
                velocity,
                sample_rate,
                bit_depth,
                &layers,
            )?,
        ),
        Command::AnalyzeAll {
            output_dir,
            intent,
            note,
            hz,
            duration,
            velocity,
            sample_rate,
            bit_depth,
            layers,
        } => cmd_analyze_all(
            output_dir,
            intent,
            export_params(
                note,
                hz,
                duration,
                velocity,
                sample_rate,
                bit_depth,
                &layers,
            )?,
        ),
    }
}

fn export_params(
    note: Option<u8>,
    hz: Option<f64>,
    duration: Option<f64>,
    velocity: f32,
    sample_rate: u32,
    bit_depth: u16,
    layers: &str,
) -> SynthResult<ExportParams> {
    Ok(ExportParams {
        note,
        hz,
        duration,
        velocity,
        sample_rate,
        bit_depth,
        layers: LayerMode::parse(layers)?,
    })
}

fn cmd_list() -> SynthResult<()> {
    println!("{:<16} {:>4} {:>6}  {}", "ID", "NOTE", "SEC", "DESCRIPTION");
    for info in factory_info()? {
        println!(
            "{:<16} {:>4} {:>6.2}  {}",
            info.id, info.default_note, info.default_duration, info.description
        );
    }
    Ok(())
}

fn cmd_algos() {
    println!("4-op algorithms (Yamaha TX81Z / DX21 numbering):\n");
    for algo in Algorithm::ALL {
        println!(
            "  {}  {:<20} {}",
            algo.id(),
            algo.name(),
            algo.description()
        );
    }
}

fn cmd_render(
    preset_name: Option<String>,
    preset_file: Option<PathBuf>,
    output: Option<PathBuf>,
    export: ExportParams,
) -> SynthResult<()> {
    let preset = match (&preset_file, &preset_name) {
        (Some(path), _) => load_preset_file(path)?,
        (None, Some(name)) => load_preset(name)?,
        (None, None) => {
            return Err(fm_synth::Error::InvalidParam {
                message: "specify --preset <name> or --preset-file <path.toml>".into(),
            });
        }
    };

    let id = output_preset_id(preset_name.as_deref(), preset_file.as_deref());
    let output = output.unwrap_or_else(|| default_wav_path(&id));
    let report = render_preset_wav(&id, &preset, &output, &export)?;
    print_wrote(&report);
    Ok(())
}

fn cmd_render_all(output_dir: PathBuf, export: ExportParams) -> SynthResult<()> {
    let batch = render_all_factory(&output_dir, &export)?;
    for report in &batch.written {
        print_wrote(report);
    }
    for (id, msg) in &batch.failures {
        eprintln!("error: preset `{id}`: {msg}");
    }
    batch.into_result().map(|_| ())
}

fn print_wrote(report: &WavRenderReport) {
    let layers = format_layers(&report.layers);
    eprintln!(
        "wrote {}  ({} Hz, {}-bit, {} samples, {} bytes PCM, preset `{}`, {:.2} Hz, {:.2}s, layers {layers})",
        report.path.display(),
        report.sample_rate,
        report.bit_depth,
        report.sample_count,
        report.pcm_bytes(),
        report.preset_name,
        report.frequency_hz,
        report.duration_secs
    );
}

fn format_layers(semitones: &[i16]) -> String {
    if matches!(semitones, [] | [0]) {
        return "unison".into();
    }
    semitones
        .iter()
        .map(|st| match st {
            0 => "0".into(),
            12 => "+12".into(),
            7 => "+7".into(),
            other => format!("{other:+}"),
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn cmd_analyze(
    wav: Option<PathBuf>,
    preset_name: Option<String>,
    preset_file: Option<PathBuf>,
    output: Option<PathBuf>,
    intent: Option<String>,
    export: ExportParams,
) -> SynthResult<()> {
    if let Some(wav_path) = wav {
        let data = read_wav(&wav_path)?;
        let stem = wav_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("wav");
        let png = output.unwrap_or_else(|| wav_path.with_extension("png"));
        let json = png.with_extension("json");
        let analysis = analyze_buffer(
            &data.samples,
            data.sample_rate,
            &AnalyzeOpts {
                source: Some(wav_path.display().to_string()),
                preset_id: Some(stem.to_string()),
                intent,
                ..AnalyzeOpts::default()
            },
        )?;
        write_analysis_bundle(&analysis, &png, &json)?;
        print_analyzed(&analysis, &png, &json, None);
        return Ok(());
    }

    let preset = match (&preset_file, &preset_name) {
        (Some(path), _) => load_preset_file(path)?,
        (None, Some(name)) => load_preset(name)?,
        (None, None) => {
            return Err(fm_synth::Error::InvalidParam {
                message: "specify --wav <file.wav>, --preset <name>, or --preset-file <path.toml>"
                    .into(),
            });
        }
    };

    let id = output_preset_id(preset_name.as_deref(), preset_file.as_deref());
    let png = output.unwrap_or_else(|| default_png_path(&id));
    let json = png.with_extension("json");
    let wav_out = png.with_extension("wav");
    let (samples, analysis) = analyze_preset(&id, &preset, &export, intent.as_deref())?;
    write_wav(
        &wav_out,
        &samples,
        WavSettings::new(export.sample_rate, export.bit_depth)?,
    )?;
    write_analysis_bundle(&analysis, &png, &json)?;
    print_analyzed(&analysis, &png, &json, Some(&wav_out));
    Ok(())
}

fn cmd_analyze_all(
    output_dir: PathBuf,
    intent: Option<String>,
    export: ExportParams,
) -> SynthResult<()> {
    let batch = analyze_all_factory(&output_dir, &export, intent.as_deref())?;
    for report in &batch.written {
        print_analyze_write(report);
    }
    for (id, msg) in &batch.failures {
        eprintln!("error: preset `{id}`: {msg}");
    }
    batch.into_result().map(|_| ())
}

fn print_analyzed(analysis: &Analysis, png: &Path, json: &Path, wav: Option<&Path>) {
    if let Some(wav) = wav {
        eprintln!("wrote {}", wav.display());
    }
    let r = &analysis.report;
    let pitch = match &r.pitch {
        Some(p) => format!(
            "{:.0}->{:.0} Hz ({:+.1} st)",
            p.start_hz, p.end_hz, p.drop_semitones
        ),
        None => "n/a".into(),
    };
    let layers = if r.render_layers.is_empty() {
        String::new()
    } else {
        format!(", layers {}", r.render_layers.join("+"))
    };
    eprintln!(
        "analyzed {}  (png {}, json {}, centroid={:.0} Hz, flatness={:.3}, pitch={}, sub={:.2}{layers})",
        r.preset_id.as_deref().unwrap_or("buffer"),
        png.display(),
        json.display(),
        r.spectral_centroid_hz,
        r.spectral_flatness,
        pitch,
        r.band_energy.sub_20_80,
    );
}

fn print_analyze_write(report: &AnalyzeWriteReport) {
    eprintln!(
        "analyzed {}  (png {}, json {})",
        report.preset_id,
        report.png.display(),
        report.json.display()
    );
}
