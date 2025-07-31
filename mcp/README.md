# MCP Servers Setup

このディレクトリには、開発に使用するMCP (Model Context Protocol) サーバーの設定が含まれています。

## セットアップ手順

### 1. 前提条件
- Docker と Docker Compose がインストールされていること
- Claude Desktop または MCP対応のクライアントがインストールされていること

### 2. 初回セットアップ

プロジェクトルートで以下のコマンドを実行：

```bash
# MCP サーバーのセットアップ
make mcp-setup
```

### 3. MCP設定の有効化

`.mcp.json`ファイルはプロジェクトルートに配置されており、以下のサーバーが設定されています：

- **context7**: ライブラリのドキュメント検索
- **serena**: インテリジェントなコード読み取りと編集

この設定は`${PWD}`環境変数を使用しているため、どのマシンでも動作します。

### 4. Serenaプロジェクトインデックスの作成

初回またはコードが大幅に変更された場合：

```bash
# Serenaのインデックスを作成
uvx --from git+https://github.com/oraios/serena serena project index
```

## 使用方法

### Context7 MCPサーバー

```bash
# 起動
make start-context7

# 停止
make stop-context7

# ログ確認
make logs-context7
```

### Serena MCPサーバー

Serenaは`.mcp.json`を通じて自動的に起動されます。手動での操作は不要です。

## トラブルシューティング

### パスの問題
- `.mcp.json`は`${PWD}`を使用しているため、必ずプロジェクトルートディレクトリから操作してください

### Dockerの権限問題
- Dockerデーモンが起動していることを確認
- 必要に応じて`docker`グループにユーザーを追加

### インデックスの更新
- コードが大幅に変更された場合は、`serena project index`を再実行してください

## 注意事項

- `.serena/`ディレクトリはGitで管理されていません（キャッシュファイルのため）
- 各開発者は自分の環境でインデックスを作成する必要があります
