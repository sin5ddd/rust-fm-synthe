# rust-fm-synthe

EDM / drum & bass 向けの **オフライン4オペFMシンセ**。プリセット（または自前のTOML）をレンダリングし、サンプラーやDAWに貼るWAVを書き出す。再生デバイスは使わない。エンジンはライブラリ、`fm-synth` は薄いCLI。

元のプレースホルダ（「FMでFX制作するツール」）を、実際にサンプルが出せるツールに置き換えた。

## できること

- 4オペレータ、Yamaha 4-op（TX81Z / DX21）系アルゴリズム 1–8
- オペレータごとに ADSR・比・デチューン・レベル・波形・固定周波数
- 1オペへのフィードバック、ピッチエンベロープ、簡易LFO、変調量スイープ
- 44.1 / 48 kHz、16 / 24-bit PCM（`hound`）
- 工場バンク: サブ、グロウル、金属ヒット、FMライザー、スタブ、ザップ、ガラスヒット

## ビルドと実行

Rust 1.74+（このリポジトリは edition 2021）。

```bash
cargo build --release
cargo run -- --help
```

サブコマンドなし、または `--help` で用法が出る。`cargo test` は一時ディレクトリにだけWAVを書く。生成WAVはコミットしない。

## コマンド例（実際にWAVが書き出される）

工場プリセット一覧:

```bash
cargo run -- list
```

アルゴリズム配線:

```bash
cargo run -- algos
```

サブのワンショット（C2、約1.35秒、16-bit / 44.1 kHz）:

```bash
cargo run -- render --preset sub-bass --output /tmp/sub-bass.wav
```

MIDIと長さを指定してグロウル:

```bash
cargo run -- render --preset growl-bass --output /tmp/growl.wav --note 36 --duration 1.8 --velocity 0.95
```

周波数指定の金属ヒット（48 kHz / 24-bit）:

```bash
cargo run -- render --preset metallic-hit --output /tmp/metallic.wav --hz 880 --duration 0.5 --sample-rate 48000 --bit-depth 24
```

ライザー（ビルド用、2.4秒）:

```bash
cargo run -- render --preset fm-riser --output /tmp/riser.wav
```

短いスタブ:

```bash
cargo run -- render --preset stab-pluck --output /tmp/stab.wav --note 64 --duration 0.4
```

自作TOML:

```bash
cargo run -- render --preset-file presets/zap.toml --output /tmp/zap.wav
```

`release` ビルドのほうが速い（ライザーなど長めのレンダー向け）:

```bash
cargo run --release -- render --preset fm-riser --output /tmp/riser.wav
```

成功すると stderr にパス、サンプル数、PCMバイト数が出る。ファイルが「全部ゼロ」ならバグなので issue にしてほしい。

## オペレータとアルゴリズム

FMではオペレータ（ここでは正弦波ベースのオシレータ）の出力で、別のオペレータの**位相**を歪める。

- **キャリア**: 出力に混ざるオペ。聞こえる音の芯。
- **モジュレータ**: キャリア（または別モジュレータ）の位相を動かす。音色・倍音・金属感を決める。
- **レシオ (ratio)**: ノート周波数に対する倍数。`1` が基音、`2` が1オクターブ上。非整数比はベルや歪み。
- **フィードバック**: 指定オペ（既定は OP4）が自分の出力で自分を変調する。上げると倍音が砂状・ノイズに近づく。
- **固定周波数 (`freq_mode = "fixed"`)**: ノートに追従しない。金属打楽器のリンに使う。

アルゴリズムは「誰が誰を変調するか」の配線。番号は Yamaha 4-op に合わせた。

| ID | 名前 | 配線 |
|----|------|------|
| 1 | serial | 4→3→2→1 |
| 2 | parallel-mod | (4+3)→2→1 |
| 3 | double-mod | 4→3→1 と 2→1 |
| 4 | shared-mod | 4→3→1 と 4→2→1 |
| 5 | dual-stack | 4→3 と 2→1 |
| 6 | triple-carrier | 4→3 / 4→2 / 4→1 |
| 7 | stack-plus-carriers | 4→3 + 2 + 1 |
| 8 | all-carriers | 4+3+2+1（加算 + OP4 FB） |

波形: `sine` / `half-sine` / `abs-sine` / `pulse`。後ろ三つはモジュレータ向き（倍音とDCが増える）。

## 工場プリセット

| ID | 用途 |
|----|------|
| `sub-bass` | キック下のサブワンショット |
| `growl-bass` | ミッドのグロウル / Reese下地 |
| `metallic-hit` | 金属パーカッション |
| `fm-riser` | ノイズ寄りの上昇FX |
| `stab-pluck` | 短いスタブ / プラック |
| `zap` | 下向きピッチのレーザー |
| `glass-hit` | ガラス／ベルの短いヒット |

データは `presets/*.toml`。同じ内容を `include_str!` でバイナリに埋め込んでいるので、クローン直後の `cargo run` でも工場バンクは使える。

## プリセットの足し方

1. `presets/stab-pluck.toml` をコピーする。
2. `name` / `description` / `algorithm`（1–8 または `serial` など）を変える。
3. `[[operators]]` を **必ず4つ**。上から OP1…OP4。
4. ワンショットは `sustain = 0`。ライザーは ` [pitch] ` と `[mod_sweep]`。
5. 試す:

```bash
cargo run -- render --preset-file presets/my-shot.toml --output /tmp/my-shot.wav --note 48
```

工場バンクに入れるなら:

- ファイルを `presets/<id>.toml` に置く
- `src/preset.rs` の `FACTORY` に `("<id>", include_str!("../presets/<id>.toml"))` を足す

主なキー:

```toml
algorithm = 1          # または "serial"
feedback = 0.4
feedback_op = 4
default_note = 36      # MIDI。CLIで省略したときの音高
default_duration = 1.2

[pitch]
start_semitones = 0.0
end_semitones = 0.0
curve = 0.0            # 正の値で後半が急（ライザー向き）

[lfo]
rate_hz = 5.5
depth_cents = 12.0

[mod_sweep]
start = 1.0            # 変調（キャリア音量ではない）の倍率
end = 1.0

[[operators]]
ratio = 1.0
detune_cents = 0.0
level = 1.0
attack = 0.005
decay = 0.3
sustain = 0.0
release = 0.08
vel_sens = 0.35
waveform = "sine"      # sine | half-sine | abs-sine | pulse
freq_mode = "ratio"    # ratio | fixed
fixed_hz = 440.0
```

## ライブラリとして使う

```rust
use fm_synth::{load_factory, render, write_wav, RenderParams, WavSettings};

let preset = load_factory("sub-bass")?;
let samples = render(
    &preset,
    &RenderParams {
        frequency_hz: 55.0,
        duration_secs: 1.2,
        velocity: 0.9,
        sample_rate: 44_100,
    },
)?;
write_wav(
    std::path::Path::new("/tmp/sub.wav"),
    &samples,
    WavSettings::new(44_100, 16)?,
)?;
```

公開APIの中心は `load_preset` / `load_factory` / `render` / `write_wav`。別ツールからエンジンだけ駆動する想定。

## テスト

```bash
cargo test
```

エンジンが無音でないこと、WAVヘッダとデータサイズ、工場プリセットのスモークを見る。WAVは `/tmp/fm_synth_tests/` など一時ディレクトリへ出す。
