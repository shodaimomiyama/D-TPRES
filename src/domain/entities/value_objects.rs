//! Union型の値オブジェクト定義
//!
//! D-TPRESシステムで使用される列挙型を定義。
//! 全てのUnion型は純粋なデータ構造として実装。

use serde::{Deserialize, Serialize};

/// プロセスが持つことができるロール
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessRole {
    /// データ所有者ロール
    /// - 秘密鍵（skO）の管理
    /// - Shamir Secret Sharingによる分割
    /// - 再暗号化キー（ReKey）の生成
    /// - アクセス制御条件の設定
    Owner,

    /// キーフラグメント保持者ロール
    /// - kFragの保持と管理
    /// - プロキシ再暗号化の実行
    /// - cFragの生成と提供
    /// - 信頼性スコアの維持
    Holder,

    /// アクセス要求者ロール
    /// - アクセス要求の発行
    /// - EVM検証の調整
    /// - cFragの収集
    /// - 閾値達成の管理
    Requester,
}

/// 秘密の状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretStatus {
    /// アクティブ状態
    /// - 通常のアクセスが可能
    /// - 再暗号化キーの生成が可能
    Active,

    /// アーカイブ済み状態
    /// - 読み取り専用アクセスのみ可能
    /// - 新規の再暗号化キー生成は不可
    Archived,

    /// 期限切れ状態
    /// - アクセス不可
    /// - 削除待ち状態
    Expired,
}

/// アクセス要求の状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessRequestStatus {
    /// 保留中
    /// - 初期状態
    /// - EVM検証待ち
    Pending,

    /// EVM検証済み
    /// - スマートコントラクトによる検証完了
    /// - Owner承認待ち
    EvmVerified,

    /// 承認済み
    /// - Ownerによる承認完了
    /// - kFrag生成可能
    Approved,

    /// 拒否
    /// - アクセス条件を満たさない
    /// - またはOwnerによる拒否
    Rejected,

    /// 完了
    /// - 全ての処理が正常終了
    /// - 秘密へのアクセス成功
    Completed,
}

/// 再暗号化キーフラグメントの状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RekeyFragmentStatus {
    /// 作成済み
    /// - kFragが生成された初期状態
    Created,

    /// 配布済み
    /// - Holderへの配布完了
    /// - 再暗号化実行可能
    Distributed,

    /// アクティブ
    /// - 再暗号化に使用可能
    Active,

    /// 消費済み
    /// - 既に使用された
    /// - 再利用不可
    Consumed,

    /// 期限切れ
    /// - 有効期限超過
    /// - 使用不可
    Expired,
}

/// 再暗号化処理の状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReencryptionStatus {
    /// 開始
    /// - 再暗号化要求受信
    /// - Holder群への要求送信中
    Initiated,

    /// 収集中
    /// - cFragを収集している状態
    /// - 閾値未達成
    Collecting,

    /// 閾値達成
    /// - 必要数のcFragを収集完了
    /// - 復号可能状態
    ThresholdMet,

    /// 完了
    /// - 全ての処理が正常終了
    Completed,

    /// 失敗
    /// - タイムアウトまたはエラー発生
    Failed,
}

/// サポートされる暗号操作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CryptoOperation {
    /// Shamir秘密分割
    /// - 秘密をn個のシェアに分割
    /// - k個のシェアで復元可能
    ShamirSplit,

    /// プロキシ再暗号化の暗号化
    /// - Umbral PREによる暗号化
    /// - カプセル生成
    PreEncrypt,

    /// 再暗号化
    /// - kFragを使用した再暗号化
    /// - cFrag生成
    ReEncrypt,

    /// 証明検証
    /// - ゼロ知識証明の検証
    /// - アクセス権限の確認
    VerifyProof,
}
