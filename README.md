# Vidmetry

Vidmetryは、動画の時間ではなく**画面領域**を切り取ることだけに集中したWindows向けデスクトップアプリです。動画を開き、プレビュー上の枠を動かし、再生／スクラブで全編を確認して、新しいファイルへ保存できます。メディア処理はPC内で完結します。

## 主な機能

- 8個のリサイズハンドルと移動可能なクロップ枠
- ピクセル単位の座標／サイズ指定、偶数スナップ、縦横比プリセット
- 再生、一時停止、ミュート、全時間スクラバー、キーボードシーク
- WebViewで再生できない素材向けの自動H.264プレビュー生成
- 進捗表示とキャンセルに対応したFFmpeg書き出し
- 元ファイルを変更せず、一時ファイル完成後に保存先へ確定

## 保存方式

| 方式 | 実フレームを縮小 | 映像再エンコード | 用途 |
|---|---:|---:|---|
| 互換MP4 | Yes | H.264 CRF 17 | 日常利用向けの推奨設定 |
| 可逆保存 | Yes | FFV1（可逆） | デコード後の画素を維持したい場合 |
| メタデータのみ | No | No | H.264/HEVCをコピーし、対応プレイヤーだけに表示領域を伝える場合 |

「メタデータのみ」はファイル容量と符号化済み画素を維持しますが、クロップ情報を無視するプレイヤーがあります。通常は互換MP4、再圧縮による画素劣化を一切避けるなら可逆保存を選びます。

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

`test:integration`は生成動画から3方式を実際に書き出し、コーデックと寸法、可逆出力のフレームハッシュ、元動画のSHA-256不変を検証します。Windowsインストーラーは`src-tauri\target\release\bundle`以下に生成されます。

詳細な要件と設計は[docs/SDD.md](docs/SDD.md)、今回の検証結果は[docs/VERIFICATION.md](docs/VERIFICATION.md)を参照してください。

## License

Vidmetry本体は[MIT License](LICENSE)です。同梱するFFmpegの条件は[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)も確認してください。
