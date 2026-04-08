# HandTrackingMouse

> WebカメラだけでPC操作を試したい開発者・研究者向けに、手のランドマーク推論をリアルタイム表示して入力UI開発を加速するハンドトラッキング基盤。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)]()

---

## Overview

既存のハンドトラッキング環境は、セットアップが重い・依存関係が複雑・挙動確認まで時間がかかるという課題がありました。特に「まずカメラ映像上で手のランドマーク推論が安定して回るか」を素早く検証したい段階で、開発体験が悪くなりがちです。

このプロジェクトは、**「ハンドトラッキング検証の初速が遅い」という課題を、Rust製の軽量パイプラインとONNX Runtimeによる推論実行で解決する**ことを目的にしています。

競合と比べた強みは、以下の3点です。

1. Rustベースで処理フローを明確に分割し、実験コードから実運用コードへ拡張しやすい。
2. カメラ列挙・選択・設定保存までを内包し、再起動時の再設定コストを削減できる。
3. 推論失敗時にNoop処理へフォールバックし、開発中の停止リスクを抑えられる。

### Features

* ONNX Runtime + 最適化レベル設定で、256x256入力の手ランドマーク推論をリアルタイム処理
* カメラ選択UIと設定永続化により、初回選択後はスムーズに再開可能
* `camera` / `inference` / `pipeline` / `ui` / `config` のモジュール分離で拡張・保守しやすい構成

---

## Demo

以下に主要機能が分かるデモを配置してください。

![HandTrackingMouse Demo](docs/demo.gif)

操作フロー: 起動してカメラ番号を選ぶと、プレビュー上に手ランドマーク骨格が重畳表示されます。

```text
[Insert Demo Image or GIF]
```

---

## Quick Start

### Requirements

* 言語 / ランタイム: Rust (Edition 2024)
* 必要ツール: Cargo, Git, Webカメラ
* 推奨環境: Windows 11

### Installation

```bash
git clone https://github.com/yourname/hand-tracking-mouse.git
cd hand-tracking-mouse

# 依存関係を解決しつつビルド
cargo build
```

### Run

```bash
cargo run
```

起動後に表示されるカメラ一覧から番号を入力し、`Esc` キーで終了します。

---

## Usage

### Example

```bash
# デバッグ実行
cargo run

# 最適化ビルドで実行
cargo run --release
```

### Gesture Controls

* カーソル移動: 人差し指先端の追従
* 左クリック: 親指先端 + 人差し指先端のピンチ
* 右クリック: 親指先端 + 中指先端のピンチ

クリックは「押し込み判定 + 離脱判定 + クールダウン」でチャタリングを抑制しています。

### Configuration

初回起動後、設定ファイルに選択カメラとモデルパスが保存されます。

```toml
preferred_camera_name = "Integrated Camera"
model_path = "models/HandLandmarkDetector.onnx"

[pipeline]
click_pinch_press_ratio = 0.38
click_pinch_release_ratio = 0.52
click_cooldown_ms = 260
```

---

## Tech Stack

| Category | Technology | Reason |
| :-- | :-- | :-- |
| Frontend | minifb | 軽量なネイティブプレビュー表示を実現するため |
| Backend | Rust, anyhow | 高速実行と堅牢なエラーハンドリングのため |
| Database | - | 永続化はローカル設定ファイル(TOML)で十分なため |
| Infrastructure | ONNX Runtime (`ort`), nokhwa | モデル推論とクロスプラットフォームなカメラ入力を扱うため |

---

## Project Structure

```text
.
├── models/                     # ONNXモデル
├── src/
│   ├── app/                    # アプリ起動フローと制御
│   ├── camera/                 # カメラ列挙・接続・フレーム取得
│   ├── config/                 # 設定の読み書き(TOML)
│   ├── inference/              # ONNX推論セッションとランドマーク処理
│   ├── pipeline/               # フレーム処理・骨格描画パイプライン
│   ├── ui/                     # プレビュー表示とカメラ選択UI
│   └── main.rs                 # エントリーポイント
├── Cargo.toml                  # 依存関係とビルド設定
└── README.md
```

---

## Roadmap

* [x] カメラ入力の取得とプレビュー表示
* [x] ONNX手ランドマーク推論の統合
* [x] ランドマーク骨格の重畳描画
* [x] ポインタ移動・クリック等のOSマウス制御
* [ ] ジェスチャー認識の安定化としきい値調整
* [ ] パフォーマンス計測と最適化（レイテンシ/FPS改善）
* [ ] ユニットテスト・統合テストの追加

---

## License

MIT License