# リリース手順

この文書はVidmetryの保守担当者向けです。通常の開発環境とローカル検証は[CONTRIBUTING.md](../CONTRIBUTING.md)、リリース設計の根拠は[SDD.md](SDD.md)を参照してください。

## 事前確認

1. `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`のバージョンを一致させます。
2. [VERIFICATION.md](VERIFICATION.md)を対象バージョンのローカル検証結果と生成物に合わせて更新します。
3. `npm run verify`、UI/Rust/統合テスト、ライセンス監査、`npm run tauri build`を完了します。
4. リポジトリが公開状態で、固定済みFFmpeg対応ソース資産を一般利用者が取得できることを確認します。

## アプリケーションの公開

バージョンと同じ注釈付きタグを作成してプッシュします。

```powershell
git tag -a v0.4.7 -m "Vidmetry v0.4.7"
git push origin v0.4.7
```

`vX.Y.Z`タグによりReleaseワークフローが起動します。タグと各バージョンファイルが一致しない場合は公開されません。ハイフンを含むタグ（例: `v0.4.7-beta.1`）はプレリリースとして扱われます。

ワークフローは固定済みFFmpeg対応ソースのSHA-256、依存関係とライセンス、テスト、MPL依存ソース、MSI／セットアップEXEを検証します。FFmpeg対応ソースの圧縮ファイルとSHA-256を含む必要資産が下書きReleaseに揃い、リポジトリの公開状態を再確認した場合だけReleaseを公開します。失敗した下書きは公開しません。

## FFmpegエンジンの更新

FFmpegエンジンまたは対応ソースを決める入力が変わった場合だけ、専用のFFmpeg source Releaseワークフローを実行します。このワークフローは固定された公開ビルド定義とソースから完全対応ソースを組み立てて監査し、エンジン固有タグの変更不能なRelease資産として保存します。

通常のアプリケーションリリースでは対応ソースを再生成せず、`scripts/ffmpeg-sidecars.json`に固定された資産を再検証して利用します。バイナリ、ソース、ライセンスの対応関係は[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)にも反映してください。

## GitHub Actionsの保守

外部Actionは完全なコミットSHAへ固定します。Dependabotが提出する更新では、上流のRelease内容と権限差分を確認してから固定SHAを更新してください。
