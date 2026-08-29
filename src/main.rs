use clap::{Parser, Subcommand};
use fm_synth::{
    factory_info, load_preset, load_preset_file, pcm_data_bytes, render, resolve_frequency,
    write_wav, Algorithm, RenderParams, WavSettings,
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
        /// 出力WAVパス
        #[arg(short, long)]
        output: PathBuf,
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
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> fm_synth::Result<()> {
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
            note,
            hz,
            duration,
            velocity,
            sample_rate,
            bit_depth,
        ),
    }
}

fn cmd_list() -> fm_synth::Result<()> {
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
    output: PathBuf,
    note: Option<u8>,
    hz: Option<f64>,
    duration: Option<f64>,
    velocity: f32,
    sample_rate: u32,
    bit_depth: u16,
) -> fm_synth::Result<()> {
    let preset = match (preset_file, preset_name) {
        (Some(path), _) => load_preset_file(&path)?,
        (None, Some(name)) => load_preset(&name)?,
        (None, None) => {
            return Err(fm_synth::Error::InvalidParam {
                message: "specify --preset <name> or --preset-file <path.toml>".into(),
            });
        }
    };

    let frequency_hz = resolve_frequency(&preset, note, hz)?;
    let duration_secs = duration.unwrap_or(preset.default_duration);
    let params = RenderParams {
        frequency_hz,
        duration_secs,
        velocity,
        sample_rate,
    };
    let samples = render(&preset, &params)?;
    let settings = WavSettings::new(sample_rate, bit_depth)?;
    write_wav(&output, &samples, settings)?;

    let data_bytes = pcm_data_bytes(samples.len(), bit_depth, 1);
    eprintln!(
        "wrote {}  ({} Hz, {}-bit, {} samples, {} bytes PCM, preset `{}`, {:.2} Hz, {:.2}s)",
        output.display(),
        sample_rate,
        bit_depth,
        samples.len(),
        data_bytes,
        preset.name,
        frequency_hz,
        duration_secs
    );
    Ok(())
}
