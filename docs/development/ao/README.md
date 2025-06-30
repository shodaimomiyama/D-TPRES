# AOドキュメント

このディレクトリには、AO（Arweave Operations）プラットフォームとD-TPRESがどのように統合されるかについての技術文書が含まれています。

## コンテンツ

### コアドキュメント

- **[AOプロセスモデルとステートレス実行](./ao_process_model.md)**  
  AOのステートレス実行モデル、Compute Units（CU）、状態管理戦略を理解するための包括的なガイド。AOプロセスを扱う開発者にとって必読の資料です。

### カバーされる主要概念

1. **ステートレス実行モデル**
   - メッセージ間でのメモリ永続性なし
   - メッセージ駆動アーキテクチャ
   - Compute Unitの分散

2. **プロセス管理**
   - WASMモジュールによるプロセススポーン
   - Arweaveからのモジュールロード
   - マルチロールプロセス設計

3. **状態管理**
   - Arweaveへの明示的な状態永続化
   - グローバル状態としてのProcessEntity
   - 遅延ロードパターン

4. **パフォーマンス最適化**
   - バッチ操作
   - メッセージスコープキャッシング
   - 状態サイズ管理

## 外部リソース

- [AOネットワーク](https://ao.arweave.net/) - 公式AOプラットフォーム
- [Arweave](https://www.arweave.org/) - 永続ストレージレイヤー
- [D-TPRESアーキテクチャ](../../PRD.md) - システム全体のアーキテクチャ

## 関連ドキュメント

- [ドメインレイヤー概要](../domain/domain_overview.md) - ドメインエンティティとAOの連携
- [リポジトリ実装](../domain/infrastructure/repository_implementations.md) - Arweaveストレージパターン
- [サービス実装](../services/) - AOプロセス実装