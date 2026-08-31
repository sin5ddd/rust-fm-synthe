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
- 工場バンク: サブ、グロウル、金属ヒット、FMライザー、スタブ、ザップ、ガラスヒット、スーパーソーベース、フィルタプラック、BPグロウル、HPエア、**キック20種（`bd-*`）**、**スネア20種（`sd-*`）**、**リード50種（`ld-*`）**、**FX50種（`fx-*`）**、**ベース15種（`bs-*`）**、**パーカッション50種（`pc-*`）**、**ドローン50種（`dr-*`）**、**爽やかパッド30種（`pf-*`）**、**キラキラパッド30種（`ps-*`）**、**プラック30種（`pl-*`）**、**エレクトリックピアノ5種（`ep-*`）**

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

`bd-*` キックバンク（808、909、フレンチコア、ガバなど20種）も同じコマンドに含まれる。出力は `dist/bd-….wav`。各プリセットの `default_note` / `default_duration` でワンショット向きの音高・長さになる。`cp-house` / `lead-fm-pluck` / `stab-fm-fifth` / `stab-fm-major` / `reese-mid` も `render-all` に含まれる（16-bit / 48 kHz のワンショット想定）。

`sd-*` スネアバンク（808、909、DnB、ジャングル、フレンチコア、ガバなど20種）も同じ。出力は `dist/sd-….wav`。尾は意図的に長め（DAW側で切る前提）。

`ld-*` リードバンク（プラック、スーパーソー、フーバー、フレンチコア、303風、クワイアなど50種）も同じ。出力は `dist/ld-….wav`。トーンリードの既定は C3（MIDI 48）。高いキャラだけ C4（60）。**既定の長さは 120 BPM・4/4 の4小節ホールド（1小節=2秒 → 4小節=8秒。短いリリーステール込みで約8.2秒）**。プラック／スタブもキーを押さえている間は鳴り続ける（ワンショットで消えない）。`presets/ld/` の既存 `lead-fm-pluck` / `stab-fm-fifth` / `stab-fm-major` / `filter-pluck` / `stab-pluck` も同じ（リネームしない）。

`fx-*` FXバンク（リバースシンバル、ライザー、インパクト、ダウンリフター、レーザー、ウーシュなど50種）も同じ。出力は `dist/fx-….wav`。ピッチのないノイズ／スイープが多い。リバースシンバルとライザーは長め（1.5–4秒、後で切る前提）。インパクトやヒットは短い。

`bs-*` ベースバンク（808サブ、暗い／明るい／ニューロReese、ウォブル、アシッド、フレンチコア、ガバ、フーバー、歪みスクエア、タイトハウス、Amenサブ、グロウル2、正弦サブ、金属FMなど15種）も同じ。出力は `dist/bs-….wav`。TOMLは `presets/bass/`。C3付近（MIDI 36–48）。既存の `sub-bass` / `growl-bass` / `reese-mid` / `supersaw-bass` はそのまま（リネームしない追加バンク）。`reese-mid` の800–1200 Hz糊とは別。

`pc-*` パーカッションバンク（ハット、オープンハット、シェイカー、タンバ、コンガ／ボンゴ、タム、カウベル、クラべ、スナップ、トライアングル、ライドFM、ウッドブロック、クラップ変種、ザップ、フォリーなど50種）も同じ。出力は `dist/pc-….wav`。TOMLは `presets/perc/`。キックは `bd-*`、スネアは `sd-*` のまま（追加のフルキックは入れない）。短いワンショット（オープンハット／ライドだけ約1秒）。既存の `cp-house` / `glass-hit` / `metallic-hit` はそのまま。

`dr-*` ドローンバンク（正弦サブ、暗いReese、低いスーパーソー、中空5度、遅いFM、ノイズBPランブル、クワイア、トレーラーブルーム、リバースホールド、水中、インダストリアル、遠いブラス、雷ベッドなど50種）も同じ。出力は `dist/dr-….wav`。TOMLは `presets/drone/`。**既定の長さは 120 BPM・4/4 の8小節ホールド（1小節=2秒 → 8小節=16秒。短いリリーステール込みで約16.2–18秒）**。キャリアのサステインは高く、t=14秒でも聞こえる（0.4秒で消えるワンショットではない）。音高はだいたい C1–C2（MIDI 24–36）。ミッドドローンだけ C3（48）。既存の `bd-*` / `sd-*` / `ld-*` / `fx-*` / `bs-*` / `pc-*` はそのまま。

`pf-*` 爽やかパッド（朝のコーラス、Juno風の軽い広がり、フルートパッド、クワイア空気、開いた5度／9度、リディアン、柔らかい長三和音など30種）も同じ。出力は `dist/pf-….wav`。TOMLは `presets/pad-fresh/`。**ロングショットはドローンと同じ 8小節ホールド（約16.2–18秒）。t=14秒でも聞こえる。** ただしドローンがサブ／ランブルの床を担うので、こちらは **低域を厚くしない**。HP／カットオフでエネルギーはだいたい 150–200 Hz より上。音高は C3–C5（MIDI 48–72）。既存の `dr-*` / `bd-*` / `sd-*` / `ld-*` / `fx-*` / `bs-*` / `pc-*` はそのまま。

`ps-*` キラキラパッド（クリスタル、ベルホールド、シマー、オルゴールパッド、氷の輝き、進化するFMスパークル、遅いコーラスシャインなど30種）も同じ。出力は `dist/ps-….wav`。TOMLは `presets/pad-sparkle/`。長さは `pf-*` と同じ 16秒ホールド。**ベルワンショットではなく、高い部分音を持ったパッド。** 高域に存在感。キック／サブではない。音高は高め（C4–C5 が多い）。既存バンクは触らない。

`pl-*` プラックバンク（ハウス、フューチャーガラス、DnB、ポップナイロン、マレット、オルゴール、トランスゲート、短いスーパーソー、FMエレクトリックピアノ、ハープ、ミュートギター、箏、カリンバ、チャイム、ベースプラック、アシッド、ローファイ、アルペジオ、5度／長三和音スタブ、クリック、リバーススウェルなど30種）も同じ。出力は `dist/pl-….wav`。TOMLは `presets/pluck/`。**短いワンショット（だいたい 0.25–1.2秒。`pl-reverse-swell` だけ約1.75秒）。16秒パッドではない。** アンプ／フィルタの減衰は速く、サステインはほぼゼロ。音高は C3–C5（MIDI 48–72）。既存の `ld-house-pluck` / `ld-fm-pluck` / `lead-fm-pluck` などはリネームしない（役割が重なってもパッチは別）。

## 4オペEP実験（タインと胴）

クラシックな DX7 Rhodes は4オペFMそのものだが、失敗しやすい。`pl-fm-ep` のような短いワンショットは、C3で**サイン胴だけ**になりタインが消える。`ep-*` はエンジンを変えず、アルゴリズム5（2+2）または並列（7 / 8）で **胴キャリア（比1）** と **タイン（比2 / 比3）** を分ける。

アタックでは C3 の 1× / 2× / 3×（約131 / 262 / 392 Hz）が立つ。モジュレータはサステイン0・短いディケイなので指数が落ち、暖かい胴へ戻る。**ベルではない**（非整数 3.5 や `ld-bell-pluck` の 2.76 / 5.14 は使わない）。パッドでも、200msで消えるプラックでもない。胴のサステインは中庸、リリースは 0.3–0.8秒。既定は **C3（MIDI 48）**。

| ID | 内容 |
|----|------|
| `ep-rhodes-soft` | 柔らかいRhodes。タイン→サイン寄りの胴。約3.2秒 |
| `ep-rhodes-hard` | 指数／ベロ感を上げた咬み。短3度は焼かない（C–E–G向き） |
| `ep-wurli` | パルス／アブサインでミッドの樹皮感。Rhodesより短い（約1.8秒） |
| `ep-tine-bell` | タイン前のめり。2×/3×は強いがEPのまま（ベルプラックではない） |
| `ep-muted` | 暗いLP、タイン控えめのラウンジ |

出力は `dist/ep-….wav`。TOMLは `presets/ep/`。`factory_entry!("ep", "ep-…")` で工場バンクに載せる。`pl-fm-ep` は短いプラックのまま残す（リネームしない）。

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
cargo run -- render --preset-file presets/fx/zap.toml --output dist/zap.wav
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
| `lead-fm-pluck` | C3（MIDI 48、約130.8 Hz）のFMプラック（メロディ用。C4ではない。4小節ホールド） |
| `stab-fm-fifth` | C3の中空5度スタブ（C–Gのみ。長3度なし） |
| `stab-fm-major` | C3の長三和音スタブ（C–E–G。中空の `stab-fm-fifth` の対） |
| `reese-mid` | C3ミッドReeseの糊（800–1200 Hz。サブなし） |
| `ld-fm-pluck` | FMプラック（C3。既存 `lead-fm-pluck` とは別パッチ。4小節ホールド） |
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
| `bs-808-sub` | 長い正弦の808サブ（ベース。キックブームではない） |
| `bs-reese-dark` | 暗いフルレンジReese（サブ＋ミッド。`reese-mid` の糊ではない） |
| `bs-reese-bright` | 明るい広いReese（super-saw。既存 `supersaw-bass` よりデチューン広め） |
| `bs-reese-neuro` | ニューロReese（非整数比。BPは300–700付近） |
| `bs-wobble` | ベース用ウォブル（低く、サブ〜ミッド） |
| `bs-acid` | 303風ベース（`ld-acid` より低い） |
| `bs-frenchcore` | フレンチコアのミッドベース（羊毛サブなし） |
| `bs-gabber` | ガバの歪みミッドベース |
| `bs-hoover` | フーバーベース（`ld-hoover` より低い） |
| `bs-dist-square` | 歪んだスクエア／パルスベース |
| `bs-house-tight` | タイトなハウスベース（短い、サイドチェイン向き） |
| `bs-amen-sub` | Amen横のDnBサブ（`sub-bass` より短い） |
| `bs-growl-2` | 2つ目のグロウル（既存 `growl-bass` / `bp-growl` とは別） |
| `bs-sine-sub` | クリーンな正弦サブ（クリックほぼなし） |
| `bs-metal-fm` | 金属FMベース |
| `pc-hat-closed` | クローズドハット |
| `pc-hat-open` | オープンハット（尾あり） |
| `pc-hat-house` | ハウスのクローズドハット |
| `pc-hat-dnb-cl` | DnBクローズドハット |
| `pc-hat-dnb-op` | DnBオープンハット |
| `pc-hat-fc` | フレンチコアハット |
| `pc-hat-pedal` | ペダル／フットハット |
| `pc-hat-tight` | 極短いタイトハット |
| `pc-hat-dark` | 暗いクローズドハット |
| `pc-hat-noise` | ノイズ寄りのハット |
| `pc-hat-chip` | チップチューン寄りのハット |
| `pc-hat-fc-op` | フレンチコアのオープンハット |
| `pc-shaker` | シェイカー |
| `pc-shaker-short` | 短いシェイカーティック |
| `pc-tamb` | タンバリン |
| `pc-tamb-roll` | タンバリンの短いロール風 |
| `pc-cabasa` | カバサ |
| `pc-conga-hi` | ハイコンガ |
| `pc-conga-lo` | ローコンガ |
| `pc-bongo-hi` | ハイボンゴ |
| `pc-bongo-lo` | ローボンゴ |
| `pc-tom-hi` | ハイタム（`bd-*` ではない） |
| `pc-tom-mid` | ミッドタム |
| `pc-tom-lo` | ロータム（キックブームではない） |
| `pc-rim` | 木寄りのリム（`sd-rimshot` より金属が弱い） |
| `pc-cowbell` | カウベル |
| `pc-clave` | クラべ |
| `pc-snap` | フィンガースナップ |
| `pc-snaps` | スナップの重ね |
| `pc-snap-lo` | 低いスナップ |
| `pc-triangle` | トライアングル |
| `pc-ride-fm` | ライド寄りのFM |
| `pc-ride-bell` | ライドベル |
| `pc-woodblock` | ウッドブロック |
| `pc-agogo-hi` | アゴゴ高音 |
| `pc-agogo-lo` | アゴゴ低音 |
| `pc-tick-metal` | 金属ティック |
| `pc-tick-indust` | インダストリアルなティック |
| `pc-tick-clock` | 時計のティック |
| `pc-clap-dry` | ドライなクラップ（`cp-house` とは別） |
| `pc-clap-room` | ルーム寄りのクラップ |
| `pc-clap-gate` | ゲートしたクラップ |
| `pc-zap` | パーカッションのザップ |
| `pc-zap-lo` | 低いパーカッションザップ |
| `pc-foley-click` | フォリーのクリック |
| `pc-foley-thud` | フォリーの短いドサッ（キックではない） |
| `pc-foley-scratch` | フォリーのスクラッチ |
| `pc-chime` | 短いチャイム |
| `pc-guiro` | ギロの短いスクレイプ |
| `pc-stick` | スティックのクリック |
| `dr-sine-sub` | 正弦サブベッド（C1。20–40 Hzの床） |
| `dr-sub-octave` | サブ＋オクターブの正弦スタック |
| `dr-rumble` | パルスの低ランブル |
| `dr-reese-dark` | 暗いReeseドローン（サブ〜ミッド） |
| `dr-reese-wide` | 広めの暗いReese |
| `dr-supersaw-low` | 低いスーパーソードローン |
| `dr-pulse-rumble` | パルスの低うなり |
| `dr-fifth-hollow` | 中空5度ドローン（C+G。長3度なし） |
| `dr-octave-stack` | オクターブ重ねの重いベッド |
| `dr-minor-dark` | 暗い短3度寄り（6:5。長三和音なし） |
| `dr-fm-evolve` | 指数がゆっくり開くFM |
| `dr-fm-index` | 遅い指数スイープのFM |
| `dr-noisy-bp` | 低いBPのノイズランブル（80 Hz付近） |
| `dr-metal-distant` | 遠い金属のうなり |
| `dr-choir-low` | 低いクワイア寄りの重ねサイン |
| `dr-choir-dark` | より暗いクワイア（短3度） |
| `dr-trailer-bloom` | トレーラーヒットが開いてドローンに |
| `dr-impact-hold` | インパクトから床へ（消えない） |
| `dr-reverse-hold` | リバース風に開いてホールド |
| `dr-riser-slow` | 遅いライザーがドローンになる |
| `dr-underwater` | 水中の低いうなり |
| `dr-industrial` | 工場の低いハム |
| `dr-scifi-hum` | SFの電源ハム（固定60 Hz層） |
| `dr-horror` | ホラーの不協和ドローン |
| `dr-ambient-dark` | 暗いアンビエントパッド |
| `dr-brass-pad` | 低いブラスパッド |
| `dr-brass-distant` | 遠い金管 |
| `dr-thunder-bed` | 雷のベッド（遠雷ノイズ＋サブ） |
| `dr-dystopia` | ディストピアのハム |
| `dr-pad-dark` | 暗いパッドドローン |
| `dr-hum-grid` | 50/60 Hzの電源グリッド |
| `dr-void` | 虚空（極端に暗いLP） |
| `dr-abyss` | 深淵のランブル |
| `dr-cathedral` | 聖堂の低いドローバー（C3） |
| `dr-engine` | エンジンの回転うなり |
| `dr-reactor` | 原子炉のハム |
| `dr-ice-cave` | 氷穴のミッドドローン（C3） |
| `dr-fog` | 霧のパッド |
| `dr-warfare` | 戦争映画の床 |
| `dr-ritual` | 儀式の低い重ね（5度＋短3度） |
| `dr-ghost-choir` | 幽霊クワイア（C3） |
| `dr-metal-bed` | 金属ベッド |
| `dr-pulse-fifth` | パルスの5度ランブル |
| `dr-saw-minor` | ソーの短3度スタック |
| `dr-fm-bell-low` | 低い金属FM（遠いゴング。薄いベルではない） |
| `dr-wobble-slow` | ごく遅いウォブル（0.15 Hz） |
| `dr-formant-low` | 低いフォルマント |
| `dr-tape-hum` | テープ／機械のハム |
| `dr-storm` | 嵐のランブル |
| `dr-score-hold` | スコア／トレーラーのホールド（C3） |
| `pf-morning` | 朝のアナログコーラスパッド（軽いスーパーソー＋HP） |
| `pf-juno-air` | Juno風の広いが軽いパッド |
| `pf-chorus-wide` | 広いコーラスパッド |
| `pf-flute-pad` | フルートパッド（C5） |
| `pf-choir-air` | 柔らかいクワイアの空気 |
| `pf-fifth-open` | 開いた5度（C+G。長3度なし） |
| `pf-ninth-open` | 開いた9度 |
| `pf-lydian-sky` | リディアン（#4）の空 |
| `pf-major-soft` | 柔らかい長三和音 |
| `pf-glass-air` | ガラスの空気 |
| `pf-dawn` | 夜明けのブルーム |
| `pf-breeze` | そよ風 |
| `pf-sky-open` | 開いた空（オクターブ＋5度） |
| `pf-cloud` | 柔らかい雲 |
| `pf-horizon` | 地平線（広い5度＋9度） |
| `pf-meadow` | 草原 |
| `pf-clear-saw` | 澄んだソーパッド |
| `pf-pulse-air` | 中空のパルス空気 |
| `pf-octave-light` | 軽いオクターブ重ね（サブなし） |
| `pf-silk` | 絹のサインパッド |
| `pf-ivory` | 象牙／柔らかい鍵盤 |
| `pf-harp-air` | ハープの空気 |
| `pf-organ-light` | 軽いオルガン（HPで床なし） |
| `pf-reed-soft` | 柔らかいリード管 |
| `pf-water-air` | 水の空気 |
| `pf-alpine` | 高山の澄んだ5度 |
| `pf-spring` | 春（リディアン寄り） |
| `pf-linen` | リネンの質感 |
| `pf-halo` | ハローのクワイア空気 |
| `pf-wide-major` | 開いた長三和音（根音＋10度＋12度） |
| `ps-crystal` | クリスタルパッド |
| `ps-bell-hold` | ベルのホールド（ワンショットではない） |
| `ps-shimmer` | シマー |
| `ps-music-box` | オルゴールパッド |
| `ps-ice-shine` | 氷の輝き |
| `ps-starlight` | 星明かり |
| `ps-glitter` | グリッター |
| `ps-chime-pad` | チャイムパッド |
| `ps-fm-sparkle` | 進化するFMスパークル |
| `ps-chorus-shine` | 遅いコーラスの輝き |
| `ps-glass-bell` | ガラスベルパッド |
| `ps-celesta` | チェレスタパッド |
| `ps-prism` | プリズム |
| `ps-frost` | 霜 |
| `ps-twinkle` | トゥインクル（ホールド） |
| `ps-aurora` | オーロラ |
| `ps-diamond` | ダイヤモンド |
| `ps-silver` | 銀 |
| `ps-glisten` | きらめき |
| `ps-high-partials` | 高次奇数倍音 |
| `ps-inharmonic` | 非整数比のスパークル |
| `ps-celestial` | 天のパッド |
| `ps-spark-evolve` | 火花が育つ |
| `ps-halo-shine` | ハローの輝き |
| `ps-crystal-choir` | クリスタルクワイア |
| `ps-bell-air` | ベルの空気 |
| `ps-glock-pad` | グロッケンパッド |
| `ps-shine-fifth` | 輝く5度 |
| `ps-ice-choir` | 氷のクワイア |
| `ps-quartz` | 石英 |
| `pl-house-dry` | ドライなハウスプラック（中庸の明るさ、0.42秒） |
| `pl-house-bright` | 明るいハウスプラック（短い） |
| `pl-future-glass` | フューチャーベースのガラスFM（明るい、0.65秒） |
| `pl-dnb-tight` | タイトなDnBプラック（ミッド、極短い） |
| `pl-dnb-neuro` | ニューロの金属FMプラック（ミッド暗い、短い） |
| `pl-pop-soft` | 柔らかいポッププラック（中庸、0.62秒） |
| `pl-pop-nylon` | ポップのナイロン（やや暗い、0.52秒） |
| `pl-mallet-marimba` | 木琴マレット（木質、短い） |
| `pl-mallet-bell` | ベルマレット（明るく金属、0.72秒） |
| `pl-musicbox` | オルゴール（明るく高い、短い） |
| `pl-trance-gate` | トランスのゲートプラック（明るい、極短い） |
| `pl-supersaw-short` | 短いスーパーソー（中庸、0.36秒） |
| `pl-fm-ep` | FMエレクトリックピアノ（暖かめ、0.95秒） |
| `pl-fm-crystal` | クリスタルFM（非常に明るい、0.68秒） |
| `pl-harp-open` | 開いたハープ（明るい、1.15秒） |
| `pl-guitar-mute` | ミュートギター（暗い、極短い） |
| `pl-koto` | 箏（明るく鋭い、0.68秒） |
| `pl-kalimba` | カリンバ（金属＋木、短い） |
| `pl-chime-high` | 高いチャイム（非常に明るい、0.85秒） |
| `pl-bass-pluck` | ベースプラック（暗い、C3、短い） |
| `pl-acid-short` | 短いアシッド（レゾで明るいミッド） |
| `pl-lofi-dust` | ローファイのダスト（暗い、0.72秒） |
| `pl-ambient-soft` | アンビエントの柔らかいプラック（暗い〜中庸、1.18秒） |
| `pl-arp-minor` | アルペジオ短3度（中庸、極短い） |
| `pl-arp-major` | アルペジオ長3度（明るい、極短い） |
| `pl-stab-fifth` | 中空5度スタブ（中庸、短い） |
| `pl-stab-major` | 長三和音スタブ（明るい、短い） |
| `pl-perc-click` | クリックプラック（明るい、0.25秒） |
| `pl-reverse-swell` | リバーススウェル（中庸、1.75秒。パッドではない） |
| `pl-clav-funk` | ファンククラビ（ミッド明るい、極短い） |
| `ep-rhodes-soft` | 柔らかいRhodes（タイン2×/3×→胴。C3、約3.2秒） |
| `ep-rhodes-hard` | 硬いRhodes（指数高め。短3度なし） |
| `ep-wurli` | ウーリッツァー寄り（パルス／アブサイン、Rhodesより短い） |
| `ep-tine-bell` | タイン前のめりEP（整数2×/3×。ベルではない） |
| `ep-muted` | ミュート／ラウンジ（暗いLP） |

データは `presets/<category>/*.toml`（`bass` / `bd` / `sd` / `ld` / `fx` / `perc` / `drone` / `pad-fresh` / `pad-sparkle` / `pluck` / `ep`）。`bs-*` は `presets/bass/`、`pc-*` は `presets/perc/`、`dr-*` は `presets/drone/`、`pf-*` は `presets/pad-fresh/`、`ps-*` は `presets/pad-sparkle/`、`pl-*` は `presets/pluck/`、`ep-*` は `presets/ep/`。同じ内容を `include_str!` でバイナリに埋め込んでいるので、クローン直後の `cargo run` でも工場バンクは使える。 `--preset <id>` の ID はファイル名のまま（フォルダ名は含まない）。

## プリセットの足し方

1. `presets/ld/stab-pluck.toml` をコピーする。
2. `name` / `description` / `algorithm`（1–8 または `serial` など）を変える。
3. `[[operators]]` を **必ず4つ**。上から OP1…OP4。
4. ワンショットは `sustain = 0`。ライザーは ` [pitch] ` と `[mod_sweep]`。
5. 試す:

```bash
cargo run -- render --preset-file presets/ld/my-shot.toml --output dist/my-shot.wav --note 48
```

工場バンクに入れるなら:

- ファイルを `presets/<category>/<id>.toml` に置く（`bass` / `bd` / `sd` / `ld` / `fx` / `perc` / `drone` / `pad-fresh` / `pad-sparkle` / `pluck` / `ep`）
- `src/preset.rs` の `FACTORY` に `factory_entry!("<category>", "<id>")` を足す（例: `factory_entry!("ep", "ep-rhodes-soft")` / `factory_entry!("pluck", "pl-house-dry")` / `factory_entry!("pad-sparkle", "ps-crystal")`）

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

公開APIの中心は `load_preset` / `load_factory` / `render` / `write_wav`。一括書き出しは `render_all_factory`（工場バンクがソース。`presets/<category>/` の重複TOMLは見ない）。別ツールからエンジンだけ駆動する想定。

## テスト

```bash
cargo test
```

エンジンが無音でないこと、WAVヘッダとデータサイズ、工場プリセットのスモーク、`bd-*` キックと `sd-*` スネアがそれぞれちょうど20個で非無音、`ld-*` リードと `fx-*` FXがそれぞれちょうど50個で非無音、`bs-*` ベースがちょうど15個で非無音、`pc-*` パーカッションがちょうど50個で非無音、`dr-*` ドローンがちょうど50個で非無音、`pf-*` 爽やかパッドと `ps-*` キラキラパッドがそれぞれちょうど30個で非無音、`pl-*` プラックがちょうど30個で非無音かつ短い（既定は2秒未満。`pl-reverse-swell` だけ例外）、`ep-*` エレクトリックピアノがちょうど5個で非無音かつ1.2秒超（クリックではない）、`ep-rhodes-soft` のアタックに純正弦より強い2×/3×タインがあること、`presets/ld/` の既定レンダーが約8秒（120 BPM の4小節）で末尾0.5秒が無音でないこと、`dr-*` / `pf-*` / `ps-*` の既定レンダーが約16秒（120 BPM の8小節）で末尾1秒と t=14秒が無音でないこと、`render_all_factory` が工場IDの数だけ非無音WAVを出すこと、super-saw が正弦と違うこと、低いLPカットオフが高域を落とすことを見る。WAVは `/tmp/fm_synth_tests/` など一時ディレクトリへ出す（リポジトリの `dist/` には書かない）。
