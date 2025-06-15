# Context7 MCP Server

Context7をDockerコンテナで実行するMCPサーバー設定です。チーム開発環境で一貫性のあるContext7環境を提供します。

## 概要

Context7は高度なRAG（Retrieval-Augmented Generation）機能を持つMCPサーバーです。このセットアップではDockerを使用して：

- **環境一貫性**: チーム全体で同じContext7バージョンを使用
- **分離実行**: ホスト環境に影響を与えずに実行
- **簡単管理**: Makefileベースの簡単な操作
- **最新版自動取得**: 常に最新のContext7 MCPを使用

## 前提条件

以下がインストールされていることを確認してください：

### 必須

- [Docker Desktop](https://www.docker.com/products/docker-desktop/) または Docker Engine
- Make (ほとんどのシステムにプリインストール済み)

### 検証コマンド

```bash
# 前提条件をチェック
make check
```

## セットアップ

### 1. Dockerイメージのビルド

```bash
make build
```

### 2. 動作確認

```bash
make start
```

## 使用方法

### 基本的なコマンド

```bash
# ヘルプの表示
make help

# Dockerイメージのビルド
make build

# Context7 MCPサーバーの起動（フォアグラウンド）
make start

# バックグラウンドでサーバー起動
make up

# バックグラウンドサーバーの停止
make down

# サーバーログの表示
make logs

# サーバーの状態確認
make status

# 前提条件チェック
make check

# クリーンアップ（イメージとコンテナの削除）
make clean
```

### バックグラウンド実行

ターミナルを占有せずにサーバーを実行する場合：

```bash
# バックグラウンドで起動
make up

# ログの確認（Ctrl+Cで抜ける）
make logs

# 状態確認
make status

# 停止
make down
```

### 開発・トラブルシューティング

```bash
# 完全な再構築・再起動
make restart

# Dockerイメージの情報表示
make info
```

## Claude Codeとの統合

このMCPサーバーはルート`.mcp.json`で設定されています：

```json
{
  "mcpServers": {
    "context7": {
      "type": "stdio",
      "command": "docker",
      "args": ["run", "-i", "--rm", "context7-mcp"],
      "transportType": "stdio"
    }
  }
}
```

または、ルートMakefileから起動する場合：

```json
{
  "mcpServers": {
    "context7": {
      "type": "stdio",
      "command": "make",
      "args": ["start-context7"],
      "cwd": "mcp/context7"
    }
  }
}
```

## Context7の機能

Context7 MCPサーバーは以下の機能を提供します：

### ライブラリ解決機能

- **パッケージ名解決**: ライブラリ名からContext7互換のライブラリIDを取得
- **関連度判定**: 検索クエリに最も関連度の高いライブラリを選択
- **メタデータ活用**: ドキュメントカバレッジとトラストスコアを考慮

### ドキュメント取得機能

- **最新ドキュメント**: 指定ライブラリの最新ドキュメントを取得
- **フォーカス検索**: 特定のトピック（hooks、routingなど）に焦点を当てた検索
- **トークン制御**: 取得するドキュメントのサイズを制御

### 使用例

```javascript
// ライブラリIDの解決
mcp__context7__resolve-library-id("react")

// ドキュメントの取得
mcp__context7__get-library-docs("/facebook/react", {
  topic: "hooks", 
  tokens: 10000
})
```

## トラブルシューティング

### よくある問題

1. **Dockerが見つからない**

   ```text
   解決方法: Docker Desktopをインストールし、起動してください
   チェック: make check
   ```

2. **Dockerが起動していない**

   ```text
   解決方法: Docker Desktopを起動してください
   チェック: docker info
   ```

3. **イメージのビルドに失敗**

   ```text
   解決方法: ネットワーク接続を確認し、make clean後にmake buildを再実行
   ```

4. **Context7サーバーが応答しない**

   ```text
   解決方法: make restart で完全に再構築・再起動
   ```

### デバッグ

詳細なDockerログを確認する場合：

```bash
# コンテナの実行状況確認
docker ps

# イメージの確認
docker images context7-mcp

# ログの確認（必要に応じて）
docker logs [CONTAINER_ID]
```

## 開発

### ファイル構造

```text
context7/
├── Dockerfile          # Context7 MCPのDockerイメージ定義
├── Makefile           # ビルド・実行コマンド
└── README.md          # このファイル
```

### カスタマイズ

Context7の設定をカスタマイズする場合は、`Dockerfile`内の環境変数やコマンドライン引数を調整してください。

## セキュリティ

- Dockerコンテナは分離された環境で実行
- ホストファイルシステムへの直接アクセスはなし
- ネットワークアクセスは必要最小限に制限
- 最新のContext7 MCPを自動取得

## ライセンス

このセットアップは基盤となる`@upstash/context7-mcp`パッケージのライセンスに従います。

## チーム開発での注意点

1. **共通環境**: 全チームメンバーが同じDockerイメージを使用
2. **バージョン管理**: Dockerfile経由で特定バージョンを固定可能
3. **リソース管理**: 不要になった際は`make clean`でクリーンアップ
4. **トラブル対応**: 問題発生時は`make restart`で環境リセット
