# リリース手順

この文書はVidmetryの保守担当者向けです。通常の開発環境とローカル検証は[CONTRIBUTING.md](../CONTRIBUTING.md)、リリース設計の根拠は[SDD.md](SDD.md)を参照してください。

## 事前確認

1. Partner Centerで製品名を予約し、GitHub ActionsのRepository variablesに`MSIX_IDENTITY_NAME`、`MSIX_PUBLISHER`、`MSIX_PUBLISHER_DISPLAY_NAME`を登録します。値はPartner Centerの製品IDに表示されるPackage/Identityと完全に一致させます。`release-tags` Rulesetは`v*`タグの作成・更新・削除を`@automachete`だけに制限し、`microsoft-store-release` Environmentは`v*`タグからのStore用MSIXジョブだけを承認なしで実行します。
2. ローカルの`main`がクリーンで`origin/main`と一致していることを確認します。アプリケーションと文書のバージョンはタグ作成スクリプトが同期します。
3. [VERIFICATION.md](VERIFICATION.md)の検証内容と生成物記録を必要に応じて更新します。バージョン表記自体はタグから同期されます。
4. `npm run verify`、UI/Rust/統合テスト、ライセンス監査、単一アプリビルドからのMSIX構造検証とNSISのライブインストール検証を完了します。
5. リポジトリが公開状態で、固定済みFFmpeg対応ソース資産を一般利用者が取得できることを確認します。
6. `npm run setup:privacy`が有効で、公開ハンドルとGitHub noreplyメールだけがGit identityに使われていることを確認します。タグを含むpush前検査を回避しないでください。

## アプリケーションの公開

次のスクリプトへリリースタグを一度入力します。関連するnpm、Cargo、Tauri、SDD、検証記録のバージョンを同期してrelease commitを作成し、同じコミットを指す軽量tagと`main`をatomic pushします。tagger identityやmessageを保存する注釈付きtagは作成しません。

```powershell
$tag = Read-Host 'Release tag (vX.Y.Z)'
./scripts/create-release-tag.ps1 -Tag $tag
```

`vX.Y.Z`タグによりReleaseワークフローが起動します。ワークフロー側でもタグから全バージョン項目を再同期して検証するため、パッケージへ古いバージョンが混入しません。ハイフンを含むタグ（例: `vX.Y.Z-beta.1`）はプレリリースとして扱われます。

ワークフローは固定済みFFmpeg対応ソースのSHA-256、依存関係とライセンス、テスト、MPL依存ソース、予約済みIDを持つx64 MSIX、直接配布用x64 NSISを検証します。MSIX、NSIS、FFmpeg対応ソースの圧縮ファイル、SHA-256の4資産が下書きReleaseに揃い、リポジトリの公開状態を再確認した場合だけReleaseを公開します。失敗した下書きは公開しません。NSISのRelease資産名は常に`Vidmetry_x64-setup.exe`で、最新の安定版は`https://github.com/automachete/vidmetry/releases/latest/download/Vidmetry_x64-setup.exe`から取得できます。NSISは未署名のためSmartScreenの警告が表示される場合があり、自動更新は行いません。

公開されたReleaseと同一のMSIXをPartner Centerへ提出します。このMSIXは提出時点では未署名で、認定後にMicrosoft Storeが本番署名します。パッケージのバージョン、アーキテクチャ、Identity、Publisherを提出画面でも再確認し、Store掲載後はStore版を実機へインストールして動画の「プログラムから開く」、フォルダーの「Open with Vidmetry」、設定によるフォルダーコマンド表示切替、更新、アンインストールを確認します。

## Store公開後の自動更新

[MicrosoftのGitHub Actions手順](https://learn.microsoft.com/en-us/windows/apps/publish/msstore-dev-cli/github-actions?tabs=msix)に合わせ、Store Product IDをRepository variableの`STORE_PRODUCT_ID`へ登録します。Microsoft Entra IDとPartner Centerの認証情報はRepository variablesへ置かず、`microsoft-store-release`のEnvironment secretsへ次の名前で登録します。

- `AZURE_AD_APPLICATION_CLIENT_ID`
- `AZURE_AD_APPLICATION_SECRET`
- `AZURE_AD_TENANT_ID`
- `SELLER_ID`

`MSIX_IDENTITY_NAME`、`MSIX_PUBLISHER`、`MSIX_PUBLISHER_DISPLAY_NAME`はMSIXマニフェスト生成用のRepository variablesであり、上記の認証用Secretsとは置き換えません。自動更新ワークフローでは`microsoft/microsoft-store-apppublisher`でMicrosoft Store Developer CLIを用意し、これら4 Secretsで`msstore reconfigure`を実行した後、`STORE_PRODUCT_ID`を指定して検証済みMSIXを公開します。Microsoftの自動更新APIはStoreで公開済みの無料製品を前提とするため、初回公開には使用しません。

## FFmpegエンジンの更新

FFmpegエンジンまたは対応ソースを決める入力が変わった場合だけ、専用のFFmpeg source Releaseワークフローを実行します。このワークフローは固定された公開ビルド定義とソースから完全対応ソースを組み立てて監査し、エンジン固有タグの変更不能なRelease資産として保存します。

通常のアプリケーションリリースでは対応ソースを再生成せず、`scripts/ffmpeg-sidecars.json`に固定された資産を再検証して利用します。バイナリ、ソース、ライセンスの対応関係は[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)にも反映してください。

## GitHub Actionsの保守

外部Actionは完全なコミットSHAへ固定します。Dependabotが提出する更新では、上流のRelease内容と権限差分を確認してから固定SHAを更新してください。
