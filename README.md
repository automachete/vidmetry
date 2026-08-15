# Vidmetry

Vidmetryは、動画の時間ではなく**画面領域**を切り取ることだけに集中したWindows向けデスクトップアプリです。動画またはフォルダーを開き、プレビュー上の枠を動かし、再生／スクラブで確認して保存できます。メディア処理はPC内で完結します。

## 主な機能

- 8個のリサイズハンドルと移動可能なクロップ枠
- ピクセル単位の座標／サイズ指定、偶数スナップ、縦横比プリセット
- 再生、一時停止、ミュート、状態を記憶するループ再生、全時間スクラバー
- フォルダーの選択／ドロップと、一覧またはPage Up／Page Downによる動画切り替え
- WebViewで再生できない素材向けの自動H.264プレビュー生成
- 日本語／英語、保存方式、映像・音声・フレームレート等の共通設定
- 「書き出し」から直接「コピーして保存」、または拡張子が一致する場合の上書き「保存」
- 進捗表示とキャンセル、安全な一時ファイル確定に対応したFFmpeg書き出し

## 保存方式

| 方式 | 実フレームを縮小 | 映像再エンコード | 用途 |
|---|---:|---:|---|
| 互換MP4 | Yes | H.264/H.265 | 日常利用向け。CRF、preset、画素形式等を設定可能 |
| 可逆保存 | Yes | FFV1（可逆） | デコード後の画素を維持したい場合 |
| メタデータのみ | No | No | H.264/HEVCをコピーし、対応プレイヤーだけに表示領域を伝える場合 |

「メタデータのみ」はファイル容量と符号化済み画素を維持しますが、クロップ情報を無視するプレイヤーがあります。通常は互換MP4、再圧縮による画素劣化を避けるなら可逆保存を選びます。保存方式と詳細値は歯車アイコンから事前に設定します。

メインの「書き出し」は保存ダイアログを直接開きます。右側のメニューから「コピーして保存」と「保存」を選べます。「保存」は設定上の出力拡張子が元動画と同じ場合だけ有効になり、確認後に元ファイルを置き換えます。

## 開発環境

- Windows 11 x64
- Node.js 24系とnpm
- Rust stable（MSVC）
- Visual Studio Build ToolsのDesktop development with C++
- Microsoft Edge WebView2 Runtime

セットアップと起動:

```powershell
npm ci
.\scripts\setup-ffmpeg.ps1
npm run tauri dev
```

FFmpegスクリプトは配布元のSHA-256を検証し、Tauri用の`ffmpeg`／`ffprobe`サイドカーを配置します。バイナリはGit管理されません。

## テストとビルド

```powershell
npm run verify
cargo test --manifest-path src-tauri\Cargo.toml
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings
npm run test:integration
npm run tauri build
```

`test:integration`は生成動画から各方式を実際に書き出し、カスタムHEVC設定、コーデックと寸法、可逆出力のフレームハッシュ、上書き置換、元テスト動画のSHA-256不変を検証します。Windowsインストーラーは`src-tauri\target\release\bundle`以下に生成されます。

詳細な要件と設計は[docs/SDD.md](docs/SDD.md)、今回の検証結果は[docs/VERIFICATION.md](docs/VERIFICATION.md)を参照してください。

## License

Vidmetry本体は[MIT License](LICENSE)です。同梱するFFmpegの条件は[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)も確認してください。
