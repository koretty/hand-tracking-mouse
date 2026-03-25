# Architecture Overview

このリポジトリは Web カメラ入力を起点に、ONNX Runtime 推論とカーソル制御をつないだリアルタイム処理基盤です。

## モジュール一覧と役割

- src.main: エントリポイント。`app::run()` を呼び出して処理を開始。
- src.app: アプリ全体のライフサイクルを管理。設定読込、カメラ選択、処理パイプライン起動、FPS表示を実行。
- src.preferences: TOML設定の永続化。`AppConfig` と `PipelineConfig` のデフォルト値と保存先解決を提供。
- src.camera: 利用可能カメラの列挙とセッション管理。RGBフレーム取得を担当。
- src.inference: ONNX Runtime セッション管理。フレーム（またはROI）を入力して 21 点ランドマークを推論。
- src.pipeline: 推論ワーカーと描画・平滑化・ROI追跡・カーソル移動を統合するリアルタイム処理層。
- src.ui: カメラ選択UIとプレビューウィンドウ表示（minifb）を提供。

## 設計原則

- 主従関係: `app::run()` が制御を持ち、1フレーム単位で `camera -> pipeline -> ui` を駆動。
- 障害耐性: 推論初期化失敗時は `NoopProcessor` にフォールバックしてプレビューを継続。
- 性能: 推論は専用ワーカーへ分離し、メインループはノンブロッキングで結果を吸収。
- 拡張性: `FrameProcessor` トレイト境界で処理系を差し替え可能。

## クラス図（概要）

```mermaid
classDiagram
    class AppConfig {
        +Option~String~ preferred_camera_name
        +String model_path
        +PipelineConfig pipeline
    }

    class PipelineConfig {
        +u32 detection_warmup_frames
        +u32 lost_to_reset_roi
        +f32 roi_expand_ratio
        +f32 landmark_smooth_alpha
        +f32 cursor_smooth_alpha
        +f32 cursor_interp_alpha
        +usize index_finger_tip
        +f32 inference_hz
        +f32 cursor_update_hz
    }

    class ConfigStore {
        +new(app_name)
        +load()
        +save(config)
    }

    class CameraDevice {
        +String display_name
        +CameraIndex index
    }

    class CameraSession {
        +open(device)
        +capture_frame() Frame
    }

    class HandLandmarkSession {
        +from_model_file(model_path)
        +run_on_frame_with_roi(frame, roi)
    }

    class Frame {
        +usize width
        +usize height
        +Vec~u8~ data
    }

    class FrameProcessor {
        <<trait>>
        +process(frame) Frame
    }

    class HandTrackingProcessor {
        +new(model_path, pipeline_config)
        +process(frame) Frame
    }
    class NoopProcessor {
        +process(frame) Frame
    }

    class PreviewWindow {
        +new(title)
        +is_open()
        +render_rgb(frame)
    }

    class FpsCounter {
        +tick()
        +current_fps()
    }

    ConfigStore --> AppConfig : load/save
    AppConfig --> PipelineConfig : owns
    CameraSession --> CameraDevice : open from
    HandTrackingProcessor ..|> FrameProcessor
    NoopProcessor ..|> FrameProcessor
    HandTrackingProcessor --> HandLandmarkSession : uses worker
    HandTrackingProcessor --> Frame : input/output
    PreviewWindow --> Frame : render
    src.app.system ..> ConfigStore : uses
    src.app.system ..> CameraSession : uses
    src.app.system ..> FrameProcessor : uses
    src.app.system ..> PreviewWindow : uses
    src.app.system ..> FpsCounter : uses
```
