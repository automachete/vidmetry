# Vidmetryへのコントリビューション

Vidmetryの開発に参加するための環境構築、実装時の確認事項、ローカル検証手順をまとめています。製品要件と設計は[docs/SDD.md](docs/SDD.md)、現在の検証実績は[docs/VERIFICATION.md](docs/VERIFICATION.md)、保守担当者向けの公開手順は[docs/RELEASING.md](docs/RELEASING.md)を参照してください。

## 開発環境

- Windows 11 x64
- `.node-version`に固定したNode.js 24.12.0とnpm 11.18.0
- `rust-toolchain.toml`に固定したRust 1.97.1（MSVC）
- Visual Studio Build ToolsのDesktop development with C++
- Windows SDKのMakeAppxとSignTool
- Microsoft Edge WebView2 Runtime

## セットアップと起動

PowerShellで次を実行します。

```powershell
npm ci
npx playwright install chromium
.\scripts\setup-ffmpeg.ps1
.\scripts\setup-copyleft-sources.ps1
.\scripts\generate-third-party-licenses.ps1
npm run tauri dev
```

`setup-ffmpeg.ps1`はマニフェストで固定されたアーカイブと実行ファイルのSHA-256、構成、必要なエンコーダーを検証してから、Tauri用のFFmpeg／ffprobeサイドカーを配置します。生成されたバイナリ、通知、依存ソースはGit管理対象外です。

## 変更時の指針

- UIとフロントエンドは`src`、Tauri/Rustバックエンドは`src-tauri/src`にあります。
- 共有エラーコードを変更した場合は`npm run generate:contracts`でTypeScript/Rustの契約を再生成します。
- ユーザー向けの日本語／英語テキストはロケールリソースへ追加します。
- 要件や受け入れ条件が変わる場合は`docs/SDD.md`、実行済みの検証内容が変わる場合は`docs/VERIFICATION.md`を同じ粒度で更新します。
- 依存関係を追加・更新する場合は、JavaScriptとRustのライセンス許可リストおよび同梱通知も確認します。

## テスト

通常の変更では、まず共通検証を実行します。

```powershell
npm run verify
```

変更範囲に応じて、次の検証も実行します。

```powershell
npm run test:ui
cargo fmt --check --manifest-path src-tauri\Cargo.toml
cargo test --manifest-path src-tauri\Cargo.toml
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings
npm run test:integration
```

- `test:ui`はPlaywrightで主要操作、状態遷移、表示位置などのUI契約を検証します。
- `test:integration`は生成動画を使い、各保存方式、時間トリム、コーデック、寸法、フレーム同一性、上書き安全性を検証します。
- Rustを変更した場合はfmt、test、Clippyをすべて実行します。

ライセンスと依存関係を含む完全な確認には次を使います。

```powershell
npm run check:licenses
cargo deny --manifest-path src-tauri\Cargo.toml check licenses
cargo audit --file src-tauri\Cargo.lock
```

## ローカルビルド

```powershell
npm run tauri build -- --no-bundle
./scripts/build-msix.ps1 -SkipAppBuild
npm run test:msix
npm run tauri bundle -- --bundles nsis
npm run test:nsis -- --LiveInstall
```

GitHub直接配布用のNSISは`src-tauri\target\release\bundle\nsis`、Store提出用のMSIXは`src-tauri\target\release\bundle\msix`に生成されます。`test:nsis -- --LiveInstall`は、既存のVidmetry登録がないWindowsユーザーでインストール、動画とフォルダーの右クリック登録、設定を無効にした状態での更新、アプリ起動、アンインストールを検証します。Partner Centerの予約IDでMSIXを生成する場合は`build-msix.ps1`の`IdentityName`、`Publisher`、`PublisherDisplayName`を指定してください。`test:msix`はマニフェスト、x64実行ファイル、動画関連付け、フォルダー用COMコマンド、FFmpegおよびライセンス同梱物を展開して検証します。管理者PowerShellでは`npm run test:msix -- --LiveInstall`により署名、インストール、COM起動、アプリ起動、アンインストールまで確認できます。

## 文書の役割

| 文書 | 対象と内容 |
|---|---|
| `README.md` | エンドユーザー向けの概要、インストール、機能、操作 |
| `CONTRIBUTING.md` | 開発環境、実装時の指針、ローカル検証 |
| `docs/SDD.md` | 製品要件、設計、受け入れ条件 |
| `docs/VERIFICATION.md` | 対象バージョンで実行済みの検証結果 |
| `docs/RELEASING.md` | 保守担当者向けの公開手順とリリース安全策 |
| `THIRD_PARTY_NOTICES.md` | 同梱する第三者ソフトウェアの通知とライセンス |
