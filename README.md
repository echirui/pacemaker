# OpenPacemaker

ポータブルDJシステム **Tonium Pacemaker** のファームウェアおよびデスクトップ連携ソフトウェアのクリーンルーム再構築プロジェクトです。

---

## 🛠️ 逆アセンブル・解析に使用したツール一覧

本プロジェクトにおけるファームウェア（ARM926EJ-S / i.MX31 向け Linux 2.6 ELF バイナリ）の解析・リバースエンジニアリングには、以下のツール群を使用しています。

### 1. ファームウェア展開・パーティション抽出
- **[binwalk](https://github.com/ReFirmLabs/binwalk)**
  - `.pfw` (Pacemaker Firmware) コンテナのファイルシグネチャをスキャンし、RedBoot ブートローダー、Linux カーネル (`zImage`)、およびルートファイルシステム (`cramfs.little`) のパーティション境界を自動特定・抽出。
- **`dd` / `hexdump` / `xxd`**
  - `.pfw` ファイル末尾の 512 バイトの独自フッター領域（`PfwInfo`: バージョン番号、SW/FWコード、UUIDシグネチャ、チェックサム）の構造解析。
- **`cramfsck` / `uncramfs`**
  - Little-Endian CRAMFS 形式で圧縮格納された組み込み Linux ルートファイルシステムを展開し、`/bin/pmplayer`、`/bin/libdecoder_*.so`、起動スクリプト群を抽出。

### 2. バイナリ解析・逆アセンブル・デコンパイル
- **[Ghidra](https://ghidra-sre.org/)** (NSA)
  - ARMv5TEJ（32-bit Little Endian）アーキテクチャの静的解析および C 言語疑似コードのデコンパイル。
  - DSP アルゴリズム（Sinc 補間器、パラメトリック EQ、ルックアヘッドピークリミッター、クロスフェーダーカーブ、適応型アニーリング BPM 解析ロジック）の制御フローおよびデータ構造の解元。
- **[IDA Pro](https://hex-rays.com/ida-pro/) / [radare2](https://github.com/radareorg/radare2)**
  - ELF バイナリ内の関数境界識別、クロスリファレンス解析、呼び出しグラフの作成。
- **GNU Binutils (`arm-none-eabi-objdump` / `llvm-objdump`)**
  - `pmplayer` および各デコーダー（`libdecoder_mp3.so`, `libdecoder_aac.so` 等）の特定関数の逆アセンブルコード出力（`-d -M arm`）。
- **`readelf`**
  - ELF32 ヘッダー、プログラムヘッダー、動的リンクセクション (`.dynamic`)、依存ライブラリ、およびエクスポート/インポートシンボルテーブルの検証。
- **`nm` (`nm -C`)**
  - Itanium C++ ABI のマングル名をデコードし、内部のクラス構造（`xponaut::audioengine`, `audiocore`, `TrackAnalysedData`, `CompoundCrossfader` 等）を特定。
- **`strings`**
  - バイナリ内にハードコードされた SQLite スキーマ DDL、XML 設定タグ (`<TrackAnalysedData>`, `<PMA>`)、エラーメッセージ、および Linux デバイスノード（`/dev/ad7142`, `/dev/mc13783` 等）の抽出。

### 3. 独自開発の分析・検証プログラム (Rust)
- **[`firmware_diff`](pacemaker-core/src/bin/firmware_diff.rs)**
  - 歴代ファームウェア（13557, 13606814, 14108, 14142, 146061537, 16219, 180096069）のバイナリサイズ、デコーダーサイズ、新規導入機能フラグの自動差分比較マトリクス生成。
- **[`hardware_verifier`](pacemaker-core/src/bin/hardware_verifier.rs)**
  - 15 個の物理ボタン、AD7142 静電容量タッチホイール/フェーダー、MC13783 PMIC バッテリー監視・電源割り込み制御のロジック検証。

---

## 📂 プロジェクト構成

```
.
├── pacemaker-core/          # ファームウェアエンジン & DSP コア（Rust クリーンルーム実装）
│   ├── src/
│   │   ├── db.rs            # 組み込み SQLite エンジン（省メモリ PRAGMA 最適化）
│   │   ├── dsp/             # EQ, クロスフェーダー, リミッター, Sinc 補間器
│   │   ├── engine/          # DJ 再生・ループ・ミキサーエンジン
│   │   ├── firmware.rs      # .pfw 解析 & 実機自動検出
│   │   ├── hardware.rs      # ハードウェアボタン・センサー制御監査モジュール
│   │   └── waveform.rs      # 独自 .str (TrackAnalysedData) パーサー
│   └── src/bin/
│       ├── firmware_diff.rs # ファームウェア差分解析ツール
│       └── hardware_verifier.rs # 実機制御ロジック検証ツール
├── pmeditor-tauri/          # デスクトップ版 Pacemaker Editor (Tauri v2 + Vanilla JS/HTML5 Canvas)
│   ├── src-tauri/           # ネイティブ IPC バックエンド & 実機マスストレージ監視
│   └── ui/                  # 3バンド高精細波形表示、楽曲・プレイリスト管理 GUI
└── firmware/                # 各世代のファームウェアバイナリ (.pfw) および展開ファイル
```

---

## ⚡ ファームウェアの超低メモリ設計

実機の処理能力および搭載 RAM（Mobile DDR-SDRAM 64MB）の制約に対応するため、以下の省メモリ化を実施しています：

- **SQLite**: ページキャッシュ制限 (`cache_size = -256` / 256KB以下)、`temp_store = FILE`、`mmap_size = 0`、`threads = 0`、定期的な `PRAGMA shrink_memory;` の実行。
- **DSP Sinc 補間器**: 1次元フラット配列化によりフットプリントを 768 バイトに抑制。
- **オーディオバッファ**: `ensure_capacity` によるメモリプーリングで再生ループ中のヒープ断片化を防止。
- **ハードウェア制御**: 動的ヒープ確保を排除したゼロアロケーションイベントループ。

---

## 💻 デスクトップエディター (Pacemaker Editor) の特徴

- **実機自動検出**: USB接続された実機（`/Volumes/Pacemaker` 等）の `.Pacemaker/music.db` を自動検出・リアルタイム反映（未接続時は安全に非表示ガード）。
- **高精細 3バンド波形**: Web Audio API による 700 バー波形解析（低域: 赤/コーラル、中域: 緑/ゴールド、高域: シアン）とリアルタイム再生連動。
- **プレイリスト & CUE/LOOP 編集**: 実機のデータベースおよび `.str` ファイルへの直接書き込みと同期。
