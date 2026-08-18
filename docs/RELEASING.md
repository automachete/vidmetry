# リリース手順

この文書はVidmetryの保守担当者向けです。通常の開発環境とローカル検証は[CONTRIBUTING.md](../CONTRIBUTING.md)、リリース設計の根拠は[SDD.md](SDD.md)を参照してください。

## 事前確認

1. Partner Centerで製品名を予約し、GitHub ActionsのRepository variablesに`MSIX_IDENTITY_NAME`、`MSIX_PUBLISHER`、`MSIX_PUBLISHER_DISPLAY_NAME`を登録します。値はPartner Centerの製品IDに表示されるPackage/Identityと完全に一致させます。`microsoft-store-release` Environmentには`@automachete`の承認を必須にし、Store用MSIXジョブは承認後だけ実行します。
2. `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`のバージョンを一致させます。
3. [VERIFICATION.md](VERIFICATION.md)を対象バージョンのローカル検証結果と生成物に合わせて更新します。
4. `npm run verify`、UI/Rust/統合テスト、ライセンス監査、`npm run msix:build`、`npm run test:msix`を完了します。
5. リポジトリが公開状態で、固定済みFFmpeg対応ソース資産を一般利用者が取得できることを確認します。

## アプリケーションの公開

バージョンと同じ注釈付きタグを作成してプッシュします。

```powershell
git tag -a v0.4.7 -m "Vidmetry v0.4.7"
git push origin v0.4.7
```

`vX.Y.Z`タグによりReleaseワークフローが起動します。タグと各バージョンファイルが一致しない場合は公開されません。ハイフンを含むタグ（例: `v0.4.7-beta.1`）はプレリリースとして扱われます。

ワークフローは固定済みFFmpeg対応ソースのSHA-256、依存関係とライセンス、テスト、MPL依存ソース、予約済みIDを持つx64 MSIXを検証します。MSIX、FFmpeg対応ソースの圧縮ファイル、SHA-256の3資産が下書きReleaseに揃い、リポジトリの公開状態を再確認した場合だけReleaseを公開します。失敗した下書きは公開しません。

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
