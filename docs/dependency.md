# モジュール依存関係

プロジェクト内部のモジュール依存を示します。

```mermaid
graph TD
    main["src.main"] --> app["src.app"]

    app --> preferences["src.preferences"]
    app --> camera["src.camera"]
    app --> pipeline["src.pipeline"]
    app --> ui["src.ui"]

    camera --> pipeline
    pipeline --> inference["src.inference"]
    ui --> camera
    ui --> pipeline

    inference --> pipeline

    %% 補足: 外部ライブラリ
    anyhow["anyhow"]
    serde["serde + toml"]
    dirs["dirs"]
    nokhwa["nokhwa"]
    ort["ort (ONNX Runtime)"]
    minifb["minifb"]
    windows_sys["windows-sys"]

    app --> anyhow
    preferences --> serde
    preferences --> dirs
    camera --> nokhwa
    inference --> ort
    ui --> minifb
    pipeline --> windows_sys
```

- 循環依存はない。主経路は main → app → (camera/pipeline/ui/preferences)。
- 推論層は pipeline 経由でのみ利用し、UI層から inference へ直接依存しない。
- OSカーソル制御は pipeline 内に閉じ、`windows-sys` 依存の影響範囲を限定している。
