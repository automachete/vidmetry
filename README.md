# Vidmetry

Vidmetryは、動画の**画面領域と開始・終了時間**を素早く切り取るWindows向けデスクトップアプリです。動画またはフォルダーを開き、プレビュー上の枠とフレーム列の両端を動かし、再生／スクラブで確認して保存できます。メディア処理はPC内で完結します。

## 主な機能

- 8個のリサイズハンドルと移動可能なクロップ枠
- ピクセル単位の座標／サイズ指定、偶数スナップ、縦横比プリセット
- フレーム列上での開始／終了トリム、低速ドラッグ時のフレーム精密調整、キーボード1フレーム調整
- Windowsのライト／ダークモードとアクセントカラーへの自動追従
- 選択範囲の再生／スクラブ、一時停止、ミュート、状態を記憶するループ再生
- 開閉できるクロップ詳細／時間トリムペインと、F11全画面動画プレビュー
- フォルダーの選択／ドロップと、一覧またはPage Up／Page Downによる動画切り替え
- WebViewで再生できない素材向けの自動H.264プレビュー生成
- 日本語／英語、保存方式、映像・音声・フレームレート等の共通設定
- 拡張子が異なる場合は直接「コピーして保存」、一致する場合は単一の「保存オプション」からコピー／上書きを選択
- 保存完了通知から、出力動画を選択した状態でエクスプローラーを表示
- 進捗表示とキャンセル、安全な一時ファイル確定に対応したFFmpeg書き出し

## 保存方式

| 方式 | 実フレームを縮小 | 映像再エンコード | 用途 |
|---|---:|---:|---|
| 互換MP4 | Yes | H.264/H.265 | 日常利用向け。CRF、preset、画素形式等を設定可能 |
| 可逆保存 | Yes | FFV1（可逆） | デコード後の画素を維持したい場合 |
| メタデータのみ | No | No | H.264/HEVCをコピーし、対応プレイヤーだけに表示領域を伝える場合 |

「メタデータのみ」はファイル容量と符号化済み画素を維持しますが、クロップ情報を無視するプレイヤーがあります。通常は互換MP4、再圧縮による画素劣化を避けるなら可逆保存を選びます。保存方式と詳細値は歯車アイコンから事前に設定します。

任意フレームで正確に時間トリミングする場合は映像ストリームコピーと両立しないため、「メタデータのみ」は時間範囲を変更している間は使用できません。可逆保存では映像をFFV1、時間境界の調整が必要な音声をFLACへ可逆変換します。

設定上の出力拡張子が元動画と異なる場合は、「コピーして保存」から保存先を選ぶだけです。同じ場合は単一の「保存オプション」を開き、「コピーして保存」または確認付きの「保存」を選べます。

## キーボード操作

| キー | 操作 |
|---|---|
| Space | 再生／一時停止（時間トリムハンドルの選択中も有効） |
| ← / → | 再生位置または選択中の開始／終了位置を1フレーム調整 |
| Shift + ← / → | 同じ位置を10フレーム調整 |
| Ctrl + S | コピーして保存 |
| Ctrl + Shift + S | 元動画へ保存（対応する保存方式のみ、確認あり） |
| F11 / Esc | 全画面動画プレビューの開始／終了 |

## 開発環境

- Windows 11 x64
- Node.js 24系とnpm
- Rust stable（MSVC）
- Visual Studio Build ToolsのDesktop development with C++
- Microsoft Edge WebView2 Runtime

セットアップと起動:

```powershell
npm ci
npx playwright install chromium
.\scripts\setup-ffmpeg.ps1
npm run tauri dev
```

FFmpegスクリプトは配布元のSHA-256を検証し、Tauri用の`ffmpeg`／`ffprobe`サイドカーを配置します。バイナリはGit管理されません。

## テストとビルド

```powershell
npm run verify
npm run test:ui
cargo test --manifest-path src-tauri\Cargo.toml
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings
npm run test:integration
npm run tauri build
```

`test:ui`はPlaywrightのChromium実描画で、Windows配色、時間ハンドルの位置とキー操作、ペイン開閉、F11、保存ショートカット、通知、保存メニューを操作・画像比較します。`test:integration`は生成動画から各方式を実際に書き出し、60フレームの時間トリム、カスタムHEVC設定、コーデックと寸法、可逆出力のフレームハッシュ、上書き置換、元テスト動画のSHA-256不変を検証します。Windowsインストーラーは`src-tauri\target\release\bundle`以下に生成されます。

## リリース

`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`のバージョンを揃えてから、同じバージョンの`vX.Y.Z`タグをプッシュします。

```powershell
git tag -a v0.4.3 -m "Vidmetry v0.4.3"
git push origin v0.4.3
```

タグを契機にGitHub Actionsが全テストを再実行し、GitHub Releaseとリリースノートを作成して、Windows用MSI／セットアップEXEを添付します。タグとアプリのバージョンが一致しない場合は公開されません。ハイフンを含むタグ（例: `v0.4.3-beta.1`）はプレリリースとして扱います。

詳細な要件と設計は[docs/SDD.md](docs/SDD.md)、今回の検証結果は[docs/VERIFICATION.md](docs/VERIFICATION.md)を参照してください。

## License

Vidmetry本体は[MIT License](LICENSE)です。同梱するFFmpegの条件は[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)も確認してください。
