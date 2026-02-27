# PR 作成ルール

## 概要

このドキュメントは、GitHub Pull Request を作成する際のルールと手順を定義します。

## 手順

Working directory: `/` （ルートにいることを確認すること）

1. **差分確認:**
    - `make diff`を実行し差分詳細を得る
2. **コードレビュー:**
    2-1. 以下のルールを踏まえて既存の変更をレビューします。ルールソースは以下の通り
        - Rust Code: `.roo/rules-rust-code`
        - Rust Test: `.roo/rules-rust-test`
    2-2. レビュー結果をユーザーに報告し、Switch mode の承認を得ます
3. **実装ステータス更新:**
    - `docs/development/status.md` に今までの実装内容を反映して更新する
4. **コミット確認:**
    - 全ての変更点がローカルでコミットされているか確認します
    - コミットされていない変更がある場合は、ユーザーにコミットするか確認します
5. **Git Push:**
    - `make push`を実行
6. **PR 要約作成:**
    - `.github/pull_request_template.md` のフォーマットに従って PR の要約を作成します
    - チェックリスト項目は AI 自身で確認します (ユーザー確認は不要)
7. **PR 内容確認と一時保存:**
    - 作成した PR の内容 (要約) に修正がないかユーザーに確認します
    - 確認後、内容を一時的に `.github/tmp_pr_description.md` に保存します
8. **PR 作成コマンド実行:**
    - PR の`TITLE` と `LABELS` はユーザーに確認または指示を仰いでください。
    - `make pr TITLE="<PRのタイトル>" LABELS="<ラベル1>,<ラベル2>"` コマンドを実行します。(TITLE と LABELS はユーザーから指示を受ける必要があります)
9. **一時ファイル削除:**
    - `make delete-pr-tmp` コマンドを実行し、`.github/tmp_pr_description.md` を削除します。

## 注意事項

- 使用可能な`LABELS`一覧（ラベル名: 説明）
  - bug: Something isn't working
  - chore: Something like small fix or grunt task
  - documentation: Improvements or additions to documentation
  - duplicate: This issue or pull request already exists
  - enhancement: New feature or request
  - good first issue: Good for newcomers
  - help wanted: Extra attention is needed
  - invalid: This doesn't seem right
  - question: Further information is requested
  - refactor: Refactor
  - test: Test for code
  - wontfix: This will not be worked on

- **PR 作成アカウント:** PR 作成時には、環境変数 `BOT_GITHUB_TOKEN` に設定された GitHub トークンを使用してください。これにより、指定されたボットアカウントで PR が作成されます。(`make pr` コマンドがこの環境変数を参照するように実装されている必要があります。)
