# プロンプト適合の見方（スペクトログラム）

レンダーしたワンショットが、プリセット説明やユーザー要望（意図）と食い違っていないかを、エージェントが確認する手順。

クレートは LLM を呼ばない。`fm-synth analyze` が **軸付き PNG** と **分析 JSON** を書き、こちらが両方を読んで判定する。

判定の正本は人間の耳と既存のユニットテスト。この手順は「要望と明らかに違う音を出し続ける」のを減らすための補助。Rhodes と Wurli のような近い音色の差は、この仕組みだけでは判定しにくい。

## 手順

1. 分析する。

```bash
cargo run --release -- analyze --preset bd-808-boom --intent "長い正弦の胴と大きなピッチ落下。サブ寄り。"
# 既存 WAV なら:
cargo run --release -- analyze --wav dist/bd-808-boom.wav --intent "..."
```

2. PNG を**画像として**開く。JSON だけ読まない。
3. JSON の数値を一次根拠にする。PNG は時間方向の動き（落下・上昇・リバース）の確認用。
4. 意図の語句を下表の観測へ対応づける。
5. `match` / `partial` / `mismatch` と根拠を書く。自信が低い項目は「この画像と数値では判断できない」と明示する。

工場バンク一括:

```bash
cargo run --release -- analyze-all
```

出力は `dist/<id>.png` と `dist/<id>.json`（`dist/` は gitignore）。

## PNG の読み方

上から:

1. **波形** — アタックの鋭さ、長さ、末尾が残るか
2. **ログ周波数スペクトログラム** — 横=時間、縦=20 Hz〜Nyquist（上が高域）。キックのピッチ落下は下向きの明るい筋。ライザーは上向き。ノイズは面、トーンは線
3. **平均スペクトル** — いわゆるスペアナ。サブが厚いか、ハイパスで床が無いか

目盛り: 20 / 50 / 100 / 200 / 500 / 1k / 2k / 5k / 10k Hz。色は 0 dB（明るい）〜 −80 dB（暗い）。フッターに centroid / flatness / pitch / 帯域比がある。

## JSON の一次根拠

| フィールド | 見る内容 |
| --- | --- |
| `duration_secs` / `attack_ms` / `tail_rms` / `energy_at_frac` | プラック vs パッド。t=0.85 で残るか |
| `band_energy.sub_20_80` など | サブ、ベース、ミッド、高域の比率（合計はおおよそ 1） |
| `spectral_centroid_hz` / `spectral_rolloff_hz` | 暗い／明るい |
| `spectral_flatness` | 低い=トーン、高い=ノイズ |
| `pitch.start_hz` / `end_hz` / `drop_semitones` | 正の drop は落下、負は上昇。`null` はピッチ不明（ノイズ等） |
| `peaks_hz` | 平均スペクトルの目立つ峰 |
| `description` / `intent` | 照合する文言 |
| `category_hints` | 接頭辞から見た「よくある期待」。自動 fail ではない |

## 語句と観測

| 意図の語句 | 見る場所 |
| --- | --- |
| キック / ブーム / サブ | 20–80 Hz が大きい、centroid が低い、よくある pitch 落下 |
| サブなし / ハイパス / エア | 20–80 Hz が小さい |
| ピッチ落下 / 808 | `pitch.start_hz` が `end_hz` より大きい |
| ライザー / アップ | 時間とともに重心またはピッチが上がる（drop が負） |
| リバース | `energy_at_frac.t85` が `t20` より大きい。スペクトログラムが後半で明るい |
| ノイズ / シンバル | flatness が高い、面状 |
| プラック / 短い | duration が短い、後半 RMS が落ちる |
| ドローン / パッド / ホールド | t=0.85 でもエネルギーが残る |
| ベル / 非整数 | `peaks_hz` が整数倍から外れる |
| 中空5度 | 1× と 1.5×。長3度（1.25×）は弱い |
| リードのオクターブ重ね | `render_layers` に `unison`+`octave`（薄いときは `fifth`）。1× と 2× が**別レンダー**。比2キャリアや chorus の octave-up ではない |
| FXのオクターブ下重ね | ピッチ系は `unison`+`octave-down`+`octave-down-2`（薄いときは `octave-down-3`）。レーザーが 1/2×・1/4× の**別レンダー**。4OP内部の比0.5化ではない |

## 判定の書き方

```
verdict: match | partial | mismatch
evidence:
  - band_energy.sub_20_80 = ...
  - pitch 180 -> 42 Hz (+24 st)
  - spectrogram: 明るい筋が時間とともに下がる
uncertain:
  - 「暖かい」は画像では判断できない
```

数値と画像が食い違うときは数値を優先し、画像側の読み取りが目盛りと合っているかだけ確認する。
