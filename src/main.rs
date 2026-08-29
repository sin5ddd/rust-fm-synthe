use clap::{Parser, Subcommand};
use fm_synth::{
    default_wav_path, factory_info, load_preset, load_preset_file, output_preset_id,
    render_all_factory, render_preset_wav, Algorithm, ExportParams, Result as SynthResult,
    WavRenderReport, DEFAULT_OUTPUT_DIR,
};
use std::path::PathBuf;
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
        } => cmd_render(
            preset,
            preset_file,
            output,
            ExportParams {
                note,
                hz,
                duration,
                velocity,
                sample_rate,
                bit_depth,
            },
        ),
        Command::RenderAll {
            output_dir,
            note,
            hz,
            duration,
            velocity,
            sample_rate,
            bit_depth,
        } => cmd_render_all(
            output_dir,
            ExportParams {
                note,
                hz,
                duration,
                velocity,
                sample_rate,
                bit_depth,
            },
        ),
    }
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
    eprintln!(
        "wrote {}  ({} Hz, {}-bit, {} samples, {} bytes PCM, preset `{}`, {:.2} Hz, {:.2}s)",
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
