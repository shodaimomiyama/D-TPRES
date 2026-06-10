# FORMIX 本番稼働 (PoC MVP) — goal コマンド指示書

**作成日**: 2026-06-11
**対象**: claudecode goal コマンドによる自律実行
**親issue**: #86 [Release] PoC MVP: formix crate を実 AO HyperBEAM 向けに E2E 動作可能にする

---

## Goal

公開 HyperBEAM ノードに事前デプロイされた FORMIX コントラクトに対し、`formix` crate の
`share()` / `recover()` が E2E で成功し、その手順が README からクリーンステートで再現可能で、
issue #86 がクローズされている状態を達成する。

```rust
let result = client.share().secret(...).execute().await?;
let recovered = client.recover().secret_id(&result.secret_id).execute().await?;
assert_eq!(recovered.recovered_secret.as_slice(), b"original");
```

## 最終 DoD チェックリスト

各項目は併記された検証コマンド・期待出力で客観的に判定する。自己申告での達成判定は禁止。

| # | 条件 | 検証方法 |
|---|------|---------|
| 1 | PR #94 がマージ済み | `gh pr view 94 --json state` → `"MERGED"` |
| 2 | ローカル HyperBEAM で E2E 成功 | `cargo test --test integration -- test_hyperbeam_e2e --ignored` がパスし、復元バイト列 == 元秘密 |
| 3 | 全テストグリーン | `make all` (check + lint + test) が exit 0、unit 271+ / integration 65+ |
| 4 | 公開ノードへデプロイ済み | `ao/deploy.json` に公開ノードの `process_id` が記録され、デプロイスクリプトが再実行可能 |
| 5 | 公開ノードで E2E 成功 | E2E テストを公開ノード endpoint 設定で実行しパス |
| 6 | README 再現性 | "Production Setup (HyperBEAM)" セクションの手順のみでクリーン環境から E2E を再現できる (#91 の DoD) |
| 7 | issue クローズ | #90, #91, #86 + 作業中に起票したブロッカー issue が全てクローズ |
| 8 | スコープ外 issue の明示 | #84, #85, #60 等に「PoC MVP スコープ外、deferred」コメントを残す |

## フェーズ構成

フェーズは順番に実行する。各フェーズの完了条件を満たすまで次へ進まない。

### Phase 0 — 現状決着

1. 未コミットの `ao/scripts/deploy.js` 差分をトリアージ(必要ならコミット、不要なら破棄理由を記録)
2. PR #94 をレビューし、CI パスを確認してマージ
3. ローカル HyperBEAM E2E がクリーンステートから再現することを確認し、#90 をクローズ

### Phase 1 — ブロッカー監査

1. #76 (FormixClient::new() の gateway URL が ActionsContainer に未接続) が実ノード E2E に
   影響するか検証し、影響するなら修正してクローズ
2. ローカル E2E をクリーン環境で実行し、新規ブロッカーを洗い出す
3. 発見したブロッカーは都度 issue 化 → ブランチ → 修正 → クローズ

### Phase 2 — 公開ノードデプロイ

1. 公開 HyperBEAM ノード(forward.computer 等)の調査・選定はエージェント判断で進める
2. デプロイスクリプトを公開ノード対応に拡張する
3. **★ 唯一の停止ポイント**: JWK 秘密鍵を env に置く必要が生じた時点で、以下をまとめて
   ユーザーに報告し、設置完了まで待機する:
   - (a) 必要な env 変数名・ファイル形式・配置場所
   - (b) 費用見込み(AR トークン、ノード利用料等)
4. 鍵設置後、デプロイ → 公開ノード E2E を実行

### Phase 3 — ドキュメント & クローズ

1. #91 を実施: README "Production Setup (HyperBEAM)" 追加、`docs/status.md` 更新、
   `docs/contracts/` 更新、旧 `ao/scripts/deploy.js` (aoconnect ベース) の整理
2. #86 の全チェックボックスを確認し、#86 をクローズ
3. スコープ外 issue に deferred コメントを残す

## ワークフロールール

- issue → `feature/issue-NN-<desc>` ブランチ(development 起点)→ PR(development 宛)→
  CI パス確認 → **自己マージ** → issue クローズ(PR 本文に `Closes #NN`)
- 新規発見の不足・バグは必ず先に issue 化してからブランチを切る(トレーサビリティ確保)
- push 前に `make fmt && make lint && make test` を必須とする
- マージ後に `docs/status.md` の該当箇所を随時更新する(Phase 3 でまとめてではなく)

## 停止ポイント・エスカレーション

- 停止は Phase 2 の「JWK 秘密鍵を env に置く段階」の 1 回のみ
- 公開ノード選定・接続検証はエージェント判断で進めてよい
- 費用発生が判明した場合は鍵設置依頼と同時に報告する(停止回数は増やさない)

## スコープ外(goal 達成判定に含めない)

- #84 [GAP-1] RandAO による Holder ランダム選出(Multi-holder distribution)
- #85 [GAP-4] Requester-Process 分離
- #60 client 側 JWK wallet loading(デプロイスクリプトでの鍵利用とは別物)
- #77, #78, #79, #50, #41, #42, #17, #18(client 品質・セキュリティ堅牢化系)
- browser / フロントエンド(O-Browser / A-Browser)
- Ed25519 メッセージ署名、Access Control List

## 前提・制約(リポジトリ規約)

- Single-process (Combined role) モデル: `ContractStorageImpl::new_single_process()` を使用
- Capsule/KFrag/CFrag のシリアライゼーションは rmp_serde (`to_bytes()`/`from_bytes()`)、
  PublicKey/Payload 系は bincode(`memory/MEMORY.md` の規約に従う)
- `ao/contracts/src/` は async/await 禁止、`client/` はネットワーク I/O に async 必須
- CLAUDE.md / `.claude/rules/` のコーディング規約・コメント規約を遵守
