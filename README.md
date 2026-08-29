# rust-fm-synthe

EDM / drum & bass 向けの **オフライン4オペFMシンセ**。プリセット（または自前のTOML）をレンダリングし、サンプラーやDAWに貼るWAVを書き出す。再生デバイスは使わない。エンジンはライブラリ、`fm-synth` は薄いCLI。

元のプレースホルダ（「FMでFX制作するツール」）を、実際にサンプルが出せるツールに置き換えた。

## できること

- 4オペレータ、Yamaha 4-op（TX81Z / DX21）系アルゴリズム 1–8
- オペレータごとに ADSR・比・デチューン・レベル・波形・固定周波数
- 波形: `sine` / `half-sine` / `abs-sine` / `pulse` / `saw`（帯域制限） / `super-saw`（1オペ内の擬似スーパーソー）
- ボイス末尾の SVF フィルタ（`lowpass` / `bandpass` / `highpass`）とカットオフ ADSR
- 1オペへのフィードバック、ピッチエンベロープ、簡易LFO、変調量スイープ
- 44.1 / 48 kHz、16 / 24-bit PCM（`hound`）
- 工場バンク: サブ、グロウル、金属ヒット、FMライザー、スタブ、ザップ、ガラスヒット、スーパーソーベース、フィルタプラック、BPグロウル、HPエア、**キック20種（`bd-*`）**、**スネア20種（`sd-*`）**、**リード50種（`ld-*`）**、**FX50種（`fx-*`）**

VA / スーパーソー専用エンジンは足していない。4オペFMのまま、波形とボイスフィルタだけ増やしている。

## ビルドと実行

Rust 1.74+（このリポジトリは edition 2021）。

```bash
cargo build --release
cargo run -- --help
```

サブコマンドなし、または `--help` で用法が出る。`cargo test` は一時ディレクトリにだけWAVを書く。生成WAVの既定先は `dist/`（gitignore済み。コミットしない）。

## コマンド例（実際にWAVが書き出される）

工場プリセット一覧:

```bash
cargo run -- list
```

アルゴリズム配線:

```bash
cargo run -- algos
```

既定の出力先は **`dist/`**（無ければ作る）。`render` で `--output` を省略すると `dist/<preset-id>.wav`。`render-all` は工場バンク全部をそこに書く。

工場バンクを一括書き出し（各プリセットの `default_note` / `default_duration`）:

```bash
cargo run -- render-all
# → dist/sub-bass.wav, dist/supersaw-bass.wav, dist/bd-808-boom.wav, dist/sd-808-snap.wav, dist/ld-fm-pluck.wav, …
```

`bd-*` キックバンク（808、909、フレンチコア、ガバなど20種）も同じコマンドに含まれる。出力は `dist/bd-….wav`。各プリセットの `default_note` / `default_duration` でワンショット向きの音高・長さになる。`cp-house` / `lead-fm-pluck` / `stab-fm-fifth` / `reese-mid` も `render-all` に含まれる（16-bit / 48 kHz のワンショット想定）。

`sd-*` スネアバンク（808、909、DnB、ジャングル、フレンチコア、ガバなど20種）も同じ。出力は `dist/sd-….wav`。尾は意図的に長め（DAW側で切る前提）。

`ld-*` リードバンク（プラック、スーパーソー、フーバー、フレンチコア、303風、クワイアなど50種）も同じ。出力は `dist/ld-….wav`。トーンリードの既定は C3（MIDI 48）。高いキャラだけ C4（60）。既存の `lead-fm-pluck` / `stab-fm-fifth` はそのまま（リネームしない追加バンク）。

`fx-*` FXバンク（リバースシンバル、ライザー、インパクト、ダウンリフター、レーザー、ウーシュなど50種）も同じ。出力は `dist/fx-….wav`。ピッチのないノイズ／スイープが多い。リバースシンバルとライザーは長め（1.5–4秒、後で切る前提）。インパクトやヒットは短い。

出力ディレクトリや長さを全プリセットに上書き:

```bash
cargo run -- render-all -o /tmp/shots --duration 0.4 --note 36
```

`--note` / `--duration` / `--hz` / `--velocity` / `--sample-rate` / `--bit-depth` は `render` と同じ。指定すると全プリセットに同じ値がかかる。

サブのワンショット（C2、約1.35秒、16-bit / 44.1 kHz）。`--output` 省略時は `dist/sub-bass.wav`:

```bash
cargo run -- render --preset sub-bass
```

明示パス（既定の `dist/<id>.wav` 以外へ）:

```bash
cargo run -- render --preset sub-bass --output dist/sub-bass.wav
```

MIDIと長さを指定してグロウル:

```bash
cargo run -- render --preset growl-bass --output dist/growl.wav --note 36 --duration 1.8 --velocity 0.95
```

周波数指定の金属ヒット（48 kHz / 24-bit）:

```bash
cargo run -- render --preset metallic-hit --output dist/metallic.wav --hz 880 --duration 0.5 --sample-rate 48000 --bit-depth 24
```

ライザー（ビルド用、2.4秒）:

```bash
cargo run -- render --preset fm-riser --output dist/riser.wav
```

短いスタブ:

```bash
cargo run -- render --preset stab-pluck --output dist/stab.wav --note 64 --duration 0.4
```

擬似スーパーソーのミッドベース:

```bash
cargo run -- render --preset supersaw-bass --output dist/supersaw-bass.wav --note 36 --duration 1.4
```

カットオフ ADSR で開閉するプラック:

```bash
cargo run -- render --preset filter-pluck --output dist/filter-pluck.wav --note 60 --duration 0.55
```

バンドパスのグロウル:

```bash
cargo run -- render --preset bp-growl --output dist/bp-growl.wav --note 40 --duration 1.5
```

ハイパスのエア／ティック:

```bash
cargo run -- render --preset hp-air --output dist/hp-air.wav --note 84 --duration 0.5
```

自作TOML:

```bash
cargo run -- render --preset-file presets/zap.toml --output dist/zap.wav
```

`release` ビルドのほうが速い（ライザーなど長めのレンダー向け）:

```bash
cargo run --release -- render --preset fm-riser --output dist/riser.wav
```

成功すると stderr にパス、サンプル数、PCMバイト数が出る（`render-all` はプリセットごとに1行）。どれかが失敗したら非ゼロで終了し、失敗した ID を出す。ファイルが「全部ゼロ」ならバグなので issue にしてほしい。

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

波形:

| 値 | 内容 |
|----|------|
| `sine` | 正弦。既定。 |
| `half-sine` | 正の半波。DCあり。モジュレータ向き。 |
| `abs-sine` | 全波整流。金属／フォルマント寄り。 |
| `pulse` | 正弦の符号。ヒット向き。 |
| `saw` | 帯域制限した単鋸波（polyBLEP）。ナイーブな `phase/π-1` ではないのでベースでも使える。 |
| `super-saw` | **1オペレータ内**の擬似スーパーソー。中心を少し大きく、他をセントで散らした約7本の鋸波を足す（JP-8000寄り）。 |

`super-saw` は高い。1オペで鋸波を約7本、4オペ全部に載せると重い（オフラインなので上限は掛けていない）。`unison`（本数、既定 7）と `unison_detune`（広がりセント、既定 20）は super-saw だけが読む。古いTOMLは省略してよい。

## フィルタ（ボイス1基）

4オペのミックスの**あと**に SVF を1基だけ通す（オペごとではない）。LP / BP / HP は同じ状態から取り出す。レゾナンスを上げても NaN にしない。FXラックは無い。

`[filter]` を省略するとローパス・カットオフ約 18 kHz・エンベロープ量 0 なので、既存プリセットの音はほぼそのまま。

| キー | 意味 |
|------|------|
| `type` | `lowpass`（既定）/ `bandpass` / `highpass` |
| `cutoff` | 基準カットオフ Hz |
| `resonance` | 0–1。0 で Q≈0.7、1 で高め（発振手前でクランプ） |
| `env_amount` | カットオフ ADSR の深さ（**オクターブ**、極性可）。0 なら固定カットオフ |
| `attack` / `decay` / `sustain` / `release` | カットオフ ADSR（秒 / サステインは 0–1） |

例: `cutoff = 200`、`env_amount = 4.6`、エンベロープが 1 のときカットオフは約 5.3 kHz（プラックが開く範囲）。負の量で閉じる。

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
| `supersaw-bass` | `super-saw` の厚みミッドベース |
| `filter-pluck` | LP + カットオフ ADSR で開閉するプラック |
| `bp-growl` | バンドパスのグロウル |
| `hp-air` | ハイパスのエア／ティック |
| `bd-808-boom` | クラシック808ブーム（長い正弦＋大きなピッチ落下） |
| `bd-808-tight` | 短いトラップ寄り808 |
| `bd-909-punch` | 909風パンチ（ミッドクリック＋短い胴） |
| `bd-house-floor` | アナログハウスの4つ打ち |
| `bd-techno-thud` | 深いテクノのドサッとした胴 |
| `bd-dnb-tight` | Amen隣接のタイトなDnBキック |
| `bd-neuro-growl` | フィルタFMのグロウルキック |
| `bd-frenchcore` | フレンチコア／ハードコア（攻撃的ミッド） |
| `bd-gabber-stomp` | ガバ／インダストリアルのストンプ |
| `bd-hardstyle` | ハードスタイル（ピッチ感・逆再生っぽいスイープ） |
| `bd-lofi-dust` | 柔らかいローファイ／ダスト |
| `bd-click` | 胴なしクリック（レイヤー用） |
| `bd-sub` | サブだけ（レイヤー用） |
| `bd-metal-ping` | 金属的なFMピンキック |
| `bd-cinematic` | 長いシネマティックブーム |
| `bd-electro-zap` | 短いエレクトロのザップキック |
| `bd-808-dist` | 歪んだ808（トラップ／EDM） |
| `bd-disco-dry` | ファンキー／ディスコのドライキック |
| `bd-jungle-round` | ブレイクビーツ／ジャングルの丸いキック |
| `bd-fm-noise` | 実験的なFMノイズキック（使える砂状） |
| `sd-808-snap` | TR-808風（短いトーン＋ノイズ、尾あり） |
| `sd-909-snappy` | TR-909風（スナップ強め、胴あり） |
| `sd-pop-tight` | タイトなアコースティック／ポップ |
| `sd-fat-backbeat` | 太いバックビート |
| `sd-rimshot` | リムショット（クリック＋リン） |
| `sd-clap-snare` | クラップ寄りのスネア（純クラップではない） |
| `sd-gated-80s` | 80年代ゲート（長いノイズ。後で切る） |
| `sd-brush-dust` | ブラシ／ダストのローファイ |
| `sd-piccolo` | ピッコロ／ハイクラック |
| `sd-dnb-tight` | DnBのタイトスネア（使える尾） |
| `sd-jungle-round` | ジャングルの丸いスネア |
| `sd-neuro-growl` | ニューロ／グロウルのFMスネア |
| `sd-frenchcore` | フレンチコア／ハードコアの割れ |
| `sd-gabber-indust` | ガバ／インダストリアル |
| `sd-trap-crisp` | トラップ（クリスプ、少し長め） |
| `sd-house-disco` | ハウス／ディスコのドライ |
| `sd-metal-ping` | 金属的なFMピンスネア |
| `sd-noise-layer` | ノイズだけのレイヤー用 |
| `sd-tone-layer` | 胴／トーンだけのレイヤー用 |
| `sd-fm-long` | 実験的な長いFMスネア（使える尾） |
| `cp-house` | 短いドライなハウスクラップ（2/4専用。1 kHz付近の胴。スネア代用ではない） |
| `lead-fm-pluck` | C3（MIDI 48、約130.8 Hz）の短いFMプラック（メロディ用。C4ではない） |
| `stab-fm-fifth` | C3の中空5度スタブ（C–Gのみ。長3度なし） |
| `reese-mid` | C3ミッドReeseの糊（800–1200 Hz。サブなし） |
| `ld-fm-pluck` | 短いFMプラック（C3。既存 `lead-fm-pluck` とは別パッチ） |
| `ld-hollow-fifth` | 中空5度リード（C–Gのみ。長3度なし） |
| `ld-house-pluck` | ドライなハウスプラック |
| `ld-dnb-stab` | タイトなDnBスタブ |
| `ld-supersaw-stab` | 広いスーパーソースタブ |
| `ld-nylon` | ミュートしたナイロン寄り |
| `ld-bell-pluck` | ベルプラック（C4） |
| `ld-mallet` | マレット／木琴 |
| `ld-perc` | パーカッション寄りのリード |
| `ld-supersaw` | クラシックなスーパーソーリード |
| `ld-unison-saw` | ユニゾンソー（デチューン控え） |
| `ld-trance-gate` | トランスのゲート風（フィルタエンベ） |
| `ld-hoover` | フーバー／アルファレーン寄り |
| `ld-sync-fm` | シンク風FM（高比モジュレータ） |
| `ld-formant` | フォルマント寄りアブサイン |
| `ld-pulse` | スクエア／パルスリード |
| `ld-anthem` | アンセムソー |
| `ld-growl` | ミッドグロウルリード |
| `ld-metallic` | 金属FMリード |
| `ld-industrial` | インダストリアル |
| `ld-frenchcore` | フレンチコアスクリーム（HP/BP。サブなし） |
| `ld-gabber` | ガバリード |
| `ld-acid` | 303風ジェスチャ（LP＋レゾ＋エンベ） |
| `ld-dist-pulse` | 歪んだパルス |
| `ld-sine` | クリーンなサインリード |
| `ld-half-sine` | ハーフサインの柔らかいリード |
| `ld-choir` | クワイア寄りの重ねサイン |
| `ld-glass` | ガラス／クリスタル |
| `ld-music-box` | オルゴール（C4） |
| `ld-flute` | フルート寄り |
| `ld-organ` | オルガン（並列ドローバー） |
| `ld-fifth-pad` | 5度パッドリード |
| `ld-octave` | オクターブスタック |
| `ld-zap` | ザップリード |
| `ld-drop-pluck` | ドロッププラック |
| `ld-laser` | レーザー（C4） |
| `ld-vowel` | 母音FM |
| `ld-noisy-bp` | ノイズ寄りのBPリード |
| `ld-reverse` | リバース風ピッチエンベ |
| `ld-cinematic` | 長いシネマティックリード |
| `ld-crystal` | クリスタル（C4） |
| `ld-brass` | ブラス寄りのFMスタブ |
| `ld-reed` | リード／クラリネット寄り |
| `ld-chip` | チップチューンパルス（C4） |
| `ld-wobble` | ミッドウォブル |
| `ld-hardstyle` | ハードスタイルスクリーチ寄り（C4） |
| `ld-arp-pluck` | アルペジオ用プラック（C4） |
| `ld-saw-pluck` | ドライな単ソープラック |
| `ld-ethereal` | 空気感のあるパッドリード |
| `ld-harpsi` | ハープシコード寄り |
| `fx-rev-cym` | クラシックなリバースシンバル（暗いノイズ→開いてクラッシュ） |
| `fx-rev-crash` | 明るいリバースクラッシュ（HP） |
| `fx-rev-hat` | 短いリバースハットのスウェル |
| `fx-rev-cym-long` | 長いダークなリバースライド（3.8秒） |
| `fx-rev-cym-bright` | 短い明るいリバースシンバル（BP） |
| `fx-rev-cym-dark` | 暗いリバースシンバル（ライド寄り） |
| `fx-rev-crash-metal` | 金属FMのリバースクラッシュ |
| `fx-rev-air` | エア寄りのリバースハット |
| `fx-rev-cym-noise` | ノイズ寄りのリバースシンバル |
| `fx-rev-splash` | 短いリバーススプラッシュ |
| `fx-noise-hit` | ホワイト寄りの短いノイズヒット |
| `fx-noise-burst` | 少し長いノイズバースト |
| `fx-metal-crash` | 金属FMクラッシュ |
| `fx-glass-smash` | ガラス破砕 |
| `fx-impact` | ミッド寄りのインパクト |
| `fx-boom` | ブーム（ミッドの胴あり） |
| `fx-sub-drop` | サブドロップ |
| `fx-impact-mid` | ミッド専用インパクト |
| `fx-uplifter` | アップリフター（ピッチ＋フィルタ） |
| `fx-riser-noise` | ノイズライザー |
| `fx-riser-pitch` | ピッチ主体のライザー |
| `fx-riser-saw` | スーパーソーのライザー |
| `fx-downlifter` | ダウンリフター |
| `fx-fall` | 急なフォール |
| `fx-downlifter-noise` | ノイズのダウンリフター |
| `fx-whoosh` | ウーシュ（BP横断） |
| `fx-wind` | 風（高FBノイズ） |
| `fx-passby` | 通過音（ドップラー風） |
| `fx-laser` | 上昇レーザーFX（`ld-zap` とは別） |
| `fx-zap` | ノイズ寄りの落下ザップ |
| `fx-blip` | 極短いブリップ |
| `fx-laser-fall` | 落下レーザー |
| `fx-sweep-bp` | バンドパス掃引 |
| `fx-formant-ah` | アー母音のFXヒット |
| `fx-formant-oh` | オー母音のFXヒット |
| `fx-tape-stop` | テープストップ風 |
| `fx-rev-verb` | リバースリバーブ風（フェイク） |
| `fx-clang` | インダストリアルの金属クラング |
| `fx-frenchcore-ns` | フレンチコアのノイズスネアFX |
| `fx-gabber-stab` | ガバのスタブFX |
| `fx-crackle` | ビニールクラックルのバースト |
| `fx-radio-stab` | ラジオスタブ |
| `fx-alarm` | 短いアラーム |
| `fx-siren` | 短いサイレン風 |
| `fx-down-to-kick` | ダウンリフターからキックへ |
| `fx-trans-fill` | トランジションフィル |
| `fx-impact-dnb` | DnBインパクト |
| `fx-whoosh-hp` | ハイパスのウーシュ |
| `fx-riser-filter` | フィルタ開放のライザー |
| `fx-hoover-fall` | フーバー寄りのフォールFX |

データは `presets/*.toml`。同じ内容を `include_str!` でバイナリに埋め込んでいるので、クローン直後の `cargo run` でも工場バンクは使える。

## プリセットの足し方

1. `presets/stab-pluck.toml` をコピーする。
2. `name` / `description` / `algorithm`（1–8 または `serial` など）を変える。
3. `[[operators]]` を **必ず4つ**。上から OP1…OP4。
4. ワンショットは `sustain = 0`。ライザーは ` [pitch] ` と `[mod_sweep]`。
5. 試す:

```bash
cargo run -- render --preset-file presets/my-shot.toml --output dist/my-shot.wav --note 48
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

[filter]
type = "lowpass"       # lowpass | bandpass | highpass
cutoff = 18000.0       # Hz
resonance = 0.0        # 0–1
env_amount = 0.0       # オクターブ。正で開く、負で閉じる。0 は固定
attack = 0.0
decay = 0.0
sustain = 1.0
release = 0.05

[[operators]]
ratio = 1.0
detune_cents = 0.0
level = 1.0
attack = 0.005
decay = 0.3
sustain = 0.0
release = 0.08
vel_sens = 0.35
waveform = "sine"      # sine | half-sine | abs-sine | pulse | saw | super-saw
unison = 7             # super-saw の本数（他波形は無視）
unison_detune = 20.0   # super-saw の広がり（セント）
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

公開APIの中心は `load_preset` / `load_factory` / `render` / `write_wav`。一括書き出しは `render_all_factory`（工場バンクがソース。`presets/` の重複TOMLは見ない）。別ツールからエンジンだけ駆動する想定。

## テスト

```bash
cargo test
```

エンジンが無音でないこと、WAVヘッダとデータサイズ、工場プリセットのスモーク、`bd-*` キックと `sd-*` スネアがそれぞれちょうど20個で非無音、`ld-*` リードと `fx-*` FXがそれぞれちょうど50個で非無音、`render_all_factory` が工場IDの数だけ非無音WAVを出すこと、super-saw が正弦と違うこと、低いLPカットオフが高域を落とすことを見る。WAVは `/tmp/fm_synth_tests/` など一時ディレクトリへ出す（リポジトリの `dist/` には書かない）。
