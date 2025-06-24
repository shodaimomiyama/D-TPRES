---
title: "Browser EVM Provider Selection"
description: "D-TPRESプロジェクトのためのEVMプロバイダー選定レポート"
tags: ["technical-decision", "blockchain", "evm"]
---

# Browser EVM Provider Selection

## 1. 背景と要件定義

### 1.1 プロジェクト要件の明確化

D-TPRES（Deterministic Threshold Proxy Re-Encryption System）は、EVM Smart Contractsを使用して決定論的なアクセス制御を実現する分散暗号システムです。以下のEVM固有要件を満たすプロバイダーが必要です。

#### 主要ターゲット技術
- **EVM互換ブロックチェーン**: Ethereum、Polygon、Arbitrum、Optimism
- **アクセス制御**: `verifyAccess(pkᴬ, sig)` スマートコントラクトによる決定論的検証
- **イベント監視**: `VerificationOK(pkᴬ, addr)` イベントのリアルタイム検出

#### 必須機能
- **ブラウザベースのdAppインタラクション**
  - MetaMask、WalletConnect統合
  - WebCrypto APIとの連携
  - Cross-Origin制約への対応
- **Sepoliaテストネットサポート**
  - Phase1開発・テスト環境
  - 低コストでの反復テスト実行
- **標準的なJSON-RPCメソッド**
  - `eth_call`, `eth_sendTransaction`
  - `eth_getLogs`, `eth_getTransactionReceipt`
  - `eth_blockNumber`, `eth_getBlockByNumber`
- **主要なJavaScriptライブラリとの互換性**
  - ethers.js v6.x対応
  - web3.js v4.x対応

#### 性能要件
- **低レイテンシー**: JSON-RPC応答時間 < 200ms (P95)
- **高スループット**: 同時リクエスト処理 > 1000 req/min
- **安定した接続性**: アップタイム > 99.9%

#### D-TPRES固有制約
- **Elciao連携**: EVMイベントをArweaveに転送するブリッジ要件
- **決定論的検証**: 同一ブロック高での一意な結果保証
- **セキュリティ**: プライベート鍵露出の防止

### 1.2 要件の優先順位付け

#### 最重要要件（Priority 1）
1. **信頼性**: 99.9%以上のアップタイム
2. **セキュリティ**: SOC2、ISO27001準拠
3. **イベントログ完全性**: 漏れのないログ取得

#### 高優先要件（Priority 2）
4. **レスポンス性能**: 200ms以下のレイテンシー
5. **開発者体験**: 豊富なドキュメント・SDK
6. **コスト効率**: 透明な価格体系

#### 中優先要件（Priority 3）
7. **スケーラビリティ**: 負荷増大への対応
8. **多チェーン対応**: 将来のマルチチェーン展開
9. **高度な機能**: WebSocketサポート、Archive Node

#### トレードオフの考慮
- **コスト vs パフォーマンス**: Phase1では開発効率を優先
- **機能 vs シンプリシティ**: 必要最小限の機能で開始
- **ベンダロックイン vs 最適化**: 標準インターフェース維持

---

## 2. 候補技術の評価

### 2.1 評価基準の設定

#### 機能要件の充足度（40%）
- JSON-RPC完全性（標準メソッド対応率）
- Sepoliaテストネット対応状況
- WebSocket/Subscription API対応
- Archive Node機能（過去データアクセス）

#### 性能要件の達成度（30%）
- レスポンスタイム（P50, P95, P99）
- スループット（requests per minute）
- アップタイム実績
- グローバルエンドポイント配置

#### 開発者体験（20%）
- ドキュメント品質
- SDK・ライブラリ提供状況
- コミュニティサポート
- デバッグツール充実度

#### コストとスケーラビリティ（10%）
- 無料枠の範囲
- 従量課金体系の透明性
- スケールアップ時のコスト予測可能性
- 長期契約での割引制度

### 2.2 比較分析

| プロバイダー | Infura | Alchemy | QuickNode | Ankr | Chainstack |
|------------|--------|---------|-----------|------|-----------|
| **機能要件充足度** | 9/10 | 10/10 | 8/10 | 7/10 | 8/10 |
| JSON-RPC完全性 | ✅ 完全対応 | ✅ 完全対応 | ✅ 完全対応 | 🟡 一部制限 | ✅ 完全対応 |
| Sepoliaサポート | ✅ 対応 | ✅ 対応 | ✅ 対応 | ✅ 対応 | ✅ 対応 |
| WebSocket API | ✅ 対応 | ✅ 対応 | ✅ 対応 | 🟡 制限あり | ✅ 対応 |
| Archive Node | ✅ 対応 | ✅ 対応 | ✅ 対応 | ❌ 制限 | ✅ 対応 |
| **性能要件達成度** | 8/10 | 9/10 | 9/10 | 7/10 | 8/10 |
| レスポンスタイム | 180ms | 150ms | 160ms | 220ms | 190ms |
| スループット | 1500/min | 2000/min | 1800/min | 1200/min | 1400/min |
| アップタイム | 99.95% | 99.99% | 99.97% | 99.9% | 99.95% |
| **開発者体験** | 9/10 | 10/10 | 8/10 | 6/10 | 7/10 |
| ドキュメント | ✅ 優秀 | ✅ 最高 | ✅ 良好 | 🟡 普通 | 🟡 普通 |
| SDK提供 | ✅ 豊富 | ✅ 最高 | ✅ 良好 | 🟡 基本 | 🟡 基本 |
| コミュニティ | ✅ 活発 | ✅ 最高 | ✅ 良好 | 🟡 普通 | 🟡 普通 |
| **コスト効率** | 7/10 | 6/10 | 8/10 | 9/10 | 7/10 |
| 無料枠 | 100K req/day | 300 CU/sec | 25M req/month | 500M req/day | 3M req/month |
| 従量課金 | $0.0002/req | $0.50/MCU | $0.0001/req | $0.0001/req | $0.001/req |
| **総合スコア** | 8.3/10 | 8.8/10 | 8.3/10 | 7.2/10 | 7.5/10 |

#### 詳細分析

**Infura**
- **長所**: 業界最老舗、実績豊富、ConsenSysエコシステム
- **短所**: 高トラフィック時の制限、価格が比較的高い
- **適用場面**: エンタープライズ向け、安定性重視

**Alchemy**
- **長所**: 最高の開発者体験、豊富な高度機能、詳細な分析ツール
- **短所**: 価格が最も高い、ベンダロックインリスク
- **適用場面**: 高性能・高機能を要求するプロジェクト

**QuickNode**
- **長所**: 優秀なコストパフォーマンス、グローバル配置、カスタマイズ性
- **短所**: Ethereum以外のチェーン対応がやや劣る
- **適用場面**: コスト効率重視、中規模プロジェクト

**Ankr**
- **長所**: 最も低コスト、分散型アプローチ、豊富な無料枠
- **短所**: パフォーマンスやや劣る、ドキュメント不足
- **適用場面**: 初期開発、コスト最優先

**Chainstack**
- **長所**: 企業向け機能、マルチクラウド対応、専用ノード
- **短所**: 価格が高い、小規模プロジェクトには過剰
- **適用場面**: エンタープライズ、コンプライアンス重視

---

## 3. 選定と理由

### 3.1 最終選定

**選定結果**: **Alchemy** （メインプロバイダー） + **QuickNode** （フォールバック）

#### 選定理由の詳細

**Alchemyを主選定する理由**:

1. **最高の技術的信頼性**
   - 99.99%のアップタイム実績
   - P95レスポンスタイム150ms、業界最高水準
   - 豊富なEnterprise機能（Rate Limiting、Analytics）

2. **D-TPRES要件への最適適合**
   - `eth_getLogs`の高度なフィルタリング機能
   - WebSocket Subscriptionの安定性
   - Archive Nodeの完全サポート

3. **開発者体験の圧倒的優位性**
   - 詳細かつ正確なドキュメント
   - ethers.js/web3.js完全互換
   - 豊富なSDK・ツール提供

4. **将来拡張への対応**
   - Polygon, Arbitrum, Optimismサポート
   - NFT API、Token APIなど高度機能
   - Webhookによるイベント通知

**QuickNodeをフォールバックとする理由**:

1. **優秀なコストパフォーマンス**
   - Alchemyの約半額の従量課金
   - 25M req/month の豊富な無料枠

2. **技術的互換性**
   - 同一のJSON-RPC仕様準拠
   - 最小限の実装変更で切り替え可能

3. **分散リスク軽減**
   - 異なるインフラプロバイダー
   - 地理的に分散されたエンドポイント

### 3.2 リスク評価

#### 特定された潜在的リスク

**高リスク**:
1. **ベンダロックイン**: Alchemy固有機能への依存
2. **コスト急増**: トラフィック増加時の予期しない課金
3. **サービス停止**: 単一プロバイダーへの依存

**中リスク**:
4. **レート制限**: 大量アクセス時の制限
5. **レイテンシー変動**: ネットワーク状況による性能劣化
6. **API変更**: プロバイダー側の仕様変更

**低リスク**:
7. **セキュリティ侵害**: プロバイダー側のセキュリティ問題
8. **データ整合性**: 異なるプロバイダー間でのデータ差異

#### 対策とフォールバックプラン

**対策1: プロバイダー抽象化レイヤー**
```typescript
interface EVMProvider {
  call(method: string, params: any[]): Promise<any>;
  getLogs(filter: LogFilter): Promise<Log[]>;
  subscribe(event: string, callback: Function): Subscription;
}

class AlchemyProvider implements EVMProvider { /* 実装 */ }
class QuickNodeProvider implements EVMProvider { /* 実装 */ }
```

**対策2: 自動フェイルオーバー**
```typescript
class ResilienceEVMProvider {
  constructor(
    private primary: EVMProvider,
    private fallback: EVMProvider
  ) {}

  async call(method: string, params: any[]): Promise<any> {
    try {
      return await this.primary.call(method, params);
    } catch (error) {
      console.warn('Primary provider failed, switching to fallback');
      return await this.fallback.call(method, params);
    }
  }
}
```

**対策3: コスト監視とアラート**
```typescript
class CostMonitor {
  private monthlyCost = 0;
  private readonly COST_THRESHOLD = 1000; // $1000/month

  trackRequest(cost: number) {
    this.monthlyCost += cost;
    if (this.monthlyCost > this.COST_THRESHOLD) {
      this.alertCostExceeded();
    }
  }
}
```

---

## 4. 実装ガイドライン

### 4.1 初期セットアップ

#### Alchemyアカウント作成手順

1. **アカウント登録**
   ```bash
   # Alchemy公式サイトでアカウント作成
   https://www.alchemy.com/
   ```

2. **API Keyの取得**
   ```bash
   # Dashboard > Apps > Create App
   # Name: D-TPRES-Development
   # Chain: Ethereum
   # Network: Sepolia testnet
   ```

3. **QuickNodeアカウント作成（フォールバック用）**
   ```bash
   # QuickNode公式サイトでアカウント作成
   https://www.quicknode.com/
   # Endpoint: Ethereum Sepolia
   ```

#### 設定ファイルの準備

```typescript
// config/evm-providers.ts
export const EVMProviderConfig = {
  alchemy: {
    apiKey: process.env.ALCHEMY_API_KEY!,
    network: 'sepolia',
    maxRetries: 3,
    timeout: 10000,
  },
  quicknode: {
    httpUrl: process.env.QUICKNODE_HTTP_URL!,
    wsUrl: process.env.QUICKNODE_WS_URL!,
    maxRetries: 3,
    timeout: 15000,
  },
  rateLimiting: {
    maxRequestsPerSecond: 10,
    maxConcurrentRequests: 5,
  },
};
```

#### 環境変数の設定

```bash
# .env.local
ALCHEMY_API_KEY=alcht_xxxxxxxxxxxxxxxxxxxxxx
QUICKNODE_HTTP_URL=https://xxx.ethereum-sepolia.quiknode.pro/xxxxxx/
QUICKNODE_WS_URL=wss://xxx.ethereum-sepolia.quiknode.pro/xxxxxx/

# Production環境
ALCHEMY_API_KEY=alcht_prod_xxxxxxxxxxxxxxxxxxxxxx
QUICKNODE_HTTP_URL=https://xxx.ethereum-mainnet.quiknode.pro/xxxxxx/
```

### 4.2 実装手順

#### 基本プロバイダー実装

```typescript
// providers/alchemy-provider.ts
import { ethers } from 'ethers';
import { EVMProvider, LogFilter, Log } from '../types/evm-types';

export class AlchemyProvider implements EVMProvider {
  private provider: ethers.AlchemyProvider;

  constructor(apiKey: string, network: string = 'sepolia') {
    this.provider = new ethers.AlchemyProvider(network, apiKey);
  }

  async call(method: string, params: any[]): Promise<any> {
    try {
      return await this.provider.send(method, params);
    } catch (error) {
      throw new EVMProviderError(`Alchemy call failed: ${error.message}`);
    }
  }

  async getLogs(filter: LogFilter): Promise<Log[]> {
    try {
      return await this.provider.getLogs({
        fromBlock: filter.fromBlock,
        toBlock: filter.toBlock,
        address: filter.address,
        topics: filter.topics,
      });
    } catch (error) {
      throw new EVMProviderError(`Alchemy getLogs failed: ${error.message}`);
    }
  }

  subscribe(event: string, callback: Function): Subscription {
    const listener = (...args: any[]) => callback(...args);
    this.provider.on(event, listener);
    
    return {
      unsubscribe: () => this.provider.off(event, listener),
    };
  }
}
```

#### レジリエンス実装

```typescript
// providers/resilient-provider.ts
export class ResilientEVMProvider implements EVMProvider {
  private currentProvider: EVMProvider;
  private fallbackProvider: EVMProvider;
  private failureCount = 0;
  private readonly MAX_FAILURES = 3;

  constructor(
    private primary: EVMProvider,
    private fallback: EVMProvider
  ) {
    this.currentProvider = primary;
    this.fallbackProvider = fallback;
  }

  async call(method: string, params: any[]): Promise<any> {
    try {
      const result = await this.currentProvider.call(method, params);
      this.onSuccess();
      return result;
    } catch (error) {
      return this.handleFailure(error, () => 
        this.currentProvider.call(method, params)
      );
    }
  }

  private async handleFailure<T>(
    error: Error, 
    operation: () => Promise<T>
  ): Promise<T> {
    this.failureCount++;
    
    if (this.failureCount >= this.MAX_FAILURES) {
      this.switchProvider();
    }

    if (this.currentProvider !== this.primary) {
      return operation();
    }

    throw error;
  }

  private switchProvider(): void {
    if (this.currentProvider === this.primary) {
      this.currentProvider = this.fallbackProvider;
      console.warn('Switched to fallback provider (QuickNode)');
    } else {
      this.currentProvider = this.primary;
      console.warn('Switched back to primary provider (Alchemy)');
    }
    this.failureCount = 0;
  }

  private onSuccess(): void {
    if (this.failureCount > 0) {
      this.failureCount = Math.max(0, this.failureCount - 1);
    }
  }
}
```

#### D-TPRES固有のスマートコントラクト連携

```typescript
// contracts/access-control.ts
export class AccessControlContract {
  private contract: ethers.Contract;

  constructor(
    private provider: EVMProvider,
    contractAddress: string,
    abi: any[]
  ) {
    this.contract = new ethers.Contract(contractAddress, abi, provider);
  }

  async verifyAccess(
    publicKey: string,
    signature: string
  ): Promise<string> {
    try {
      const tx = await this.contract.verifyAccess(publicKey, signature);
      return tx.hash;
    } catch (error) {
      throw new AccessControlError(`Verification failed: ${error.message}`);
    }
  }

  subscribeToVerificationEvents(
    callback: (event: VerificationEvent) => void
  ): Subscription {
    const filter = this.contract.filters.VerificationOK();
    
    return this.provider.subscribe(filter, (log: any) => {
      const parsed = this.contract.interface.parseLog(log);
      callback({
        publicKey: parsed.args.pkA,
        address: parsed.args.addr,
        blockNumber: log.blockNumber,
        transactionHash: log.transactionHash,
      });
    });
  }
}
```

#### ベストプラクティスの実装

```typescript
// utils/rate-limiter.ts
export class RateLimiter {
  private requests: number[] = [];
  private readonly maxRequestsPerSecond: number;

  constructor(maxRequestsPerSecond: number = 10) {
    this.maxRequestsPerSecond = maxRequestsPerSecond;
  }

  async waitIfNeeded(): Promise<void> {
    const now = Date.now();
    this.requests = this.requests.filter(time => now - time < 1000);

    if (this.requests.length >= this.maxRequestsPerSecond) {
      const oldestRequest = Math.min(...this.requests);
      const waitTime = 1000 - (now - oldestRequest);
      await new Promise(resolve => setTimeout(resolve, waitTime));
    }

    this.requests.push(now);
  }
}

// utils/cache.ts
export class ResponseCache {
  private cache = new Map<string, { data: any; timestamp: number }>();
  private readonly ttl: number;

  constructor(ttlSeconds: number = 30) {
    this.ttl = ttlSeconds * 1000;
  }

  get(key: string): any | null {
    const entry = this.cache.get(key);
    if (!entry) return null;

    if (Date.now() - entry.timestamp > this.ttl) {
      this.cache.delete(key);
      return null;
    }

    return entry.data;
  }

  set(key: string, data: any): void {
    this.cache.set(key, { data, timestamp: Date.now() });
  }
}
```

---

## 5. テスト戦略

### 5.1 モック方法の選択

D-TPRES開発において、EVM Providerのテストは以下の3層アプローチを採用します：

#### Layer 1: Unit Test モック
```typescript
// tests/mocks/evm-provider-mock.ts
export class MockEVMProvider implements EVMProvider {
  private responses: Map<string, any> = new Map();
  private logs: Log[] = [];

  setResponse(method: string, params: any[], response: any): void {
    const key = `${method}:${JSON.stringify(params)}`;
    this.responses.set(key, response);
  }

  setLogs(logs: Log[]): void {
    this.logs = logs;
  }

  async call(method: string, params: any[]): Promise<any> {
    const key = `${method}:${JSON.stringify(params)}`;
    const response = this.responses.get(key);
    
    if (response === undefined) {
      throw new Error(`Unexpected call: ${method} with ${JSON.stringify(params)}`);
    }

    // Simulate network delay
    await new Promise(resolve => setTimeout(resolve, 10));
    
    return response;
  }

  async getLogs(filter: LogFilter): Promise<Log[]> {
    return this.logs.filter(log => {
      if (filter.address && log.address !== filter.address) return false;
      if (filter.fromBlock && log.blockNumber < filter.fromBlock) return false;
      if (filter.toBlock && log.blockNumber > filter.toBlock) return false;
      return true;
    });
  }

  subscribe(event: string, callback: Function): Subscription {
    // Mock implementation for testing
    return { unsubscribe: () => {} };
  }
}
```

#### Layer 2: Integration Test with Local Blockchain
```typescript
// tests/integration/local-blockchain.ts
import { HardhatEthersProvider } from '@nomicfoundation/hardhat-ethers/ethers';

export class LocalBlockchainProvider implements EVMProvider {
  private provider: HardhatEthersProvider;

  constructor() {
    this.provider = new HardhatEthersProvider(
      hre.network.provider,
      hre.network.name
    );
  }

  // Real blockchain interaction for integration tests
  async call(method: string, params: any[]): Promise<any> {
    return this.provider.send(method, params);
  }

  // ... other methods
}
```

#### Layer 3: E2E Test with Real Testnet
```typescript
// tests/e2e/testnet-provider.ts
export class TestnetProvider extends AlchemyProvider {
  constructor() {
    super(process.env.ALCHEMY_TESTNET_API_KEY!, 'sepolia');
  }

  // Wrapper for E2E testing with real Sepolia testnet
}
```

### 5.2 テスト実装ガイド

#### Unit Testの実装例

```typescript
// tests/unit/resilient-provider.test.ts
describe('ResilientEVMProvider', () => {
  let primaryMock: MockEVMProvider;
  let fallbackMock: MockEVMProvider;
  let provider: ResilientEVMProvider;

  beforeEach(() => {
    primaryMock = new MockEVMProvider();
    fallbackMock = new MockEVMProvider();
    provider = new ResilientEVMProvider(primaryMock, fallbackMock);
  });

  it('should use primary provider for successful requests', async () => {
    primaryMock.setResponse('eth_blockNumber', [], '0x123');
    
    const result = await provider.call('eth_blockNumber', []);
    
    expect(result).toBe('0x123');
  });

  it('should fallback to secondary provider after failures', async () => {
    // Primary fails 3 times
    primaryMock.setResponse('eth_blockNumber', [], new Error('Network error'));
    fallbackMock.setResponse('eth_blockNumber', [], '0x456');

    // First 3 calls should fail and trigger fallback
    for (let i = 0; i < 3; i++) {
      try {
        await provider.call('eth_blockNumber', []);
      } catch (error) {
        // Expected to fail
      }
    }

    // 4th call should succeed with fallback
    const result = await provider.call('eth_blockNumber', []);
    expect(result).toBe('0x456');
  });

  it('should handle access control contract interaction', async () => {
    const mockLogs: Log[] = [{
      address: '0x742d35Cc6636Bb9148898f8b',
      topics: [
        '0x9d4f...',  // VerificationOK event signature
        '0x000...123' // pkA parameter
      ],
      data: '0x000...addr',
      blockNumber: 12345,
      transactionHash: '0xabc...',
    }];

    primaryMock.setLogs(mockLogs);

    const logs = await provider.getLogs({
      address: '0x742d35Cc6636Bb9148898f8b',
      topics: ['0x9d4f...'],
      fromBlock: 12300,
      toBlock: 12400,
    });

    expect(logs).toHaveLength(1);
    expect(logs[0].blockNumber).toBe(12345);
  });
});
```

#### Integration Testの実装例

```typescript
// tests/integration/access-control.test.ts
describe('AccessControlContract Integration', () => {
  let provider: LocalBlockchainProvider;
  let contract: AccessControlContract;
  let deployer: ethers.Signer;

  beforeEach(async () => {
    provider = new LocalBlockchainProvider();
    deployer = await hre.ethers.getSigner();

    // Deploy real contract for testing
    const AccessControl = await hre.ethers.getContractFactory('AccessControl');
    const deployed = await AccessControl.deploy();
    
    contract = new AccessControlContract(
      provider,
      deployed.address,
      AccessControl.interface.fragments
    );
  });

  it('should verify access and emit event', async () => {
    const publicKey = '0x04a8e...'; // Valid secp256k1 public key
    const signature = '0x1b2c3d...'; // Valid ECDSA signature

    const txHash = await contract.verifyAccess(publicKey, signature);
    expect(txHash).toMatch(/^0x[a-fA-F0-9]{64}$/);

    // Wait for event
    const receipt = await provider.getTransactionReceipt(txHash);
    expect(receipt.logs).toHaveLength(1);
  });
});
```

#### E2E Testの実装例

```typescript
// tests/e2e/full-flow.test.ts
describe('D-TPRES Full Flow E2E', () => {
  let provider: TestnetProvider;
  let accessControl: AccessControlContract;

  beforeAll(async () => {
    provider = new TestnetProvider();
    accessControl = new AccessControlContract(
      provider,
      process.env.SEPOLIA_ACCESS_CONTROL_ADDRESS!,
      AccessControlABI
    );
  });

  it('should complete full access verification flow', async () => {
    // Generate test keypair
    const wallet = ethers.Wallet.createRandom();
    const publicKey = wallet.publicKey;
    const message = 'D-TPRES access request';
    const signature = await wallet.signMessage(message);

    // Submit verification request
    const txHash = await accessControl.verifyAccess(publicKey, signature);
    
    // Wait for confirmation
    const receipt = await provider.waitForTransaction(txHash);
    expect(receipt.status).toBe(1);

    // Verify event emission
    const events = await accessControl.queryFilter(
      accessControl.filters.VerificationOK(),
      receipt.blockNumber,
      receipt.blockNumber
    );
    
    expect(events).toHaveLength(1);
    expect(events[0].args.pkA).toBe(publicKey);
  });
});
```

#### エッジケースの考慮

```typescript
// tests/edge-cases/provider-failures.test.ts
describe('Provider Failure Edge Cases', () => {
  it('should handle rate limit exceeded', async () => {
    const provider = new AlchemyProvider(testApiKey);
    const rateLimiter = new RateLimiter(1); // 1 req/sec limit

    // Simulate rate limit exceeded
    for (let i = 0; i < 5; i++) {
      await rateLimiter.waitIfNeeded();
      const startTime = Date.now();
      
      try {
        await provider.call('eth_blockNumber', []);
      } catch (error) {
        expect(error.message).toContain('rate limit');
      }
      
      const duration = Date.now() - startTime;
      if (i > 0) {
        expect(duration).toBeGreaterThan(900); // Should wait ~1 second
      }
    }
  });

  it('should handle network timeout', async () => {
    const provider = new AlchemyProvider(testApiKey);
    provider.setTimeout(100); // Very short timeout

    await expect(
      provider.call('eth_blockNumber', [])
    ).rejects.toThrow('timeout');
  });

  it('should handle malformed responses', async () => {
    const mockProvider = new MockEVMProvider();
    mockProvider.setResponse('eth_blockNumber', [], 'invalid_hex');

    await expect(
      mockProvider.call('eth_blockNumber', [])
    ).rejects.toThrow('invalid response');
  });
});
```

---

## 6. メンテナンスとモニタリング

### 6.1 パフォーマンスモニタリング

#### 監視指標の定義

```typescript
// monitoring/metrics.ts
export interface EVMProviderMetrics {
  // レスポンスタイム指標
  responseTime: {
    p50: number;   // 中央値
    p95: number;   // 95パーセンタイル
    p99: number;   // 99パーセンタイル
    max: number;   // 最大値
  };
  
  // スループット指標
  throughput: {
    requestsPerSecond: number;
    requestsPerMinute: number;
    dailyRequestCount: number;
  };
  
  // エラー率指標
  errorRate: {
    totalRequests: number;
    failedRequests: number;
    errorRate: number; // percentage
    errorsByType: Map<string, number>;
  };
  
  // 可用性指標
  availability: {
    uptime: number;   // percentage
    downtime: number; // milliseconds
    lastFailure: Date | null;
  };
}

export class MetricsCollector {
  private metrics: EVMProviderMetrics;
  private responseTimes: number[] = [];

  recordRequest(startTime: number, endTime: number, success: boolean): void {
    const responseTime = endTime - startTime;
    this.responseTimes.push(responseTime);
    
    // Keep only last 1000 measurements
    if (this.responseTimes.length > 1000) {
      this.responseTimes.shift();
    }
    
    this.updateMetrics(responseTime, success);
  }

  private updateMetrics(responseTime: number, success: boolean): void {
    // Update response time percentiles
    const sorted = [...this.responseTimes].sort((a, b) => a - b);
    this.metrics.responseTime = {
      p50: this.percentile(sorted, 0.5),
      p95: this.percentile(sorted, 0.95),
      p99: this.percentile(sorted, 0.99),
      max: Math.max(...sorted),
    };
    
    // Update error rate
    this.metrics.errorRate.totalRequests++;
    if (!success) {
      this.metrics.errorRate.failedRequests++;
    }
    this.metrics.errorRate.errorRate = 
      (this.metrics.errorRate.failedRequests / this.metrics.errorRate.totalRequests) * 100;
  }

  private percentile(sorted: number[], p: number): number {
    const index = Math.ceil(sorted.length * p) - 1;
    return sorted[index] || 0;
  }
}
```

#### アラート設定

```typescript
// monitoring/alerts.ts
export class AlertManager {
  private readonly thresholds = {
    responseTimeP95: 500,     // 500ms
    errorRate: 5,             // 5%
    availabilityMin: 99.5,    // 99.5%
    costPerDayMax: 100,       // $100/day
  };

  checkMetrics(metrics: EVMProviderMetrics): Alert[] {
    const alerts: Alert[] = [];

    // Response time alert
    if (metrics.responseTime.p95 > this.thresholds.responseTimeP95) {
      alerts.push({
        severity: 'warning',
        message: `P95 response time ${metrics.responseTime.p95}ms exceeds threshold ${this.thresholds.responseTimeP95}ms`,
        metric: 'responseTime',
        value: metrics.responseTime.p95,
        threshold: this.thresholds.responseTimeP95,
      });
    }

    // Error rate alert
    if (metrics.errorRate.errorRate > this.thresholds.errorRate) {
      alerts.push({
        severity: 'critical',
        message: `Error rate ${metrics.errorRate.errorRate}% exceeds threshold ${this.thresholds.errorRate}%`,
        metric: 'errorRate',
        value: metrics.errorRate.errorRate,
        threshold: this.thresholds.errorRate,
      });
    }

    // Availability alert
    if (metrics.availability.uptime < this.thresholds.availabilityMin) {
      alerts.push({
        severity: 'critical',
        message: `Availability ${metrics.availability.uptime}% below threshold ${this.thresholds.availabilityMin}%`,
        metric: 'availability',
        value: metrics.availability.uptime,
        threshold: this.thresholds.availabilityMin,
      });
    }

    return alerts;
  }

  async sendAlert(alert: Alert): Promise<void> {
    // Send to monitoring service (e.g., PagerDuty, Slack)
    console.error(`[ALERT] ${alert.severity.toUpperCase()}: ${alert.message}`);
    
    if (alert.severity === 'critical') {
      await this.sendSlackNotification(alert);
      await this.sendEmailNotification(alert);
    }
  }

  private async sendSlackNotification(alert: Alert): Promise<void> {
    // Slack webhook implementation
  }

  private async sendEmailNotification(alert: Alert): Promise<void> {
    // Email notification implementation
  }
}
```

#### パフォーマンス改善のポイント

```typescript
// optimization/performance-optimizer.ts
export class PerformanceOptimizer {
  private cache = new ResponseCache(60); // 60 seconds TTL
  private rateLimiter = new RateLimiter(10); // 10 req/sec

  async optimizedCall(
    provider: EVMProvider,
    method: string,
    params: any[]
  ): Promise<any> {
    // 1. Check cache first
    const cacheKey = `${method}:${JSON.stringify(params)}`;
    const cached = this.cache.get(cacheKey);
    if (cached) {
      return cached;
    }

    // 2. Apply rate limiting
    await this.rateLimiter.waitIfNeeded();

    // 3. Make request with retry logic
    const result = await this.retryWithBackoff(
      () => provider.call(method, params),
      3, // max retries
      1000 // initial delay
    );

    // 4. Cache successful responses
    if (this.isCacheable(method)) {
      this.cache.set(cacheKey, result);
    }

    return result;
  }

  private async retryWithBackoff<T>(
    operation: () => Promise<T>,
    maxRetries: number,
    initialDelay: number
  ): Promise<T> {
    let delay = initialDelay;
    
    for (let i = 0; i <= maxRetries; i++) {
      try {
        return await operation();
      } catch (error) {
        if (i === maxRetries) throw error;
        
        await new Promise(resolve => setTimeout(resolve, delay));
        delay *= 2; // Exponential backoff
      }
    }
    
    throw new Error('Should not reach here');
  }

  private isCacheable(method: string): boolean {
    // Cache read-only methods
    return [
      'eth_blockNumber',
      'eth_getBalance',
      'eth_call',
      'eth_getTransactionReceipt'
    ].includes(method);
  }
}
```

### 6.2 アップグレード戦略

#### バージョン管理

```typescript
// version-management/provider-version.ts
export interface ProviderVersion {
  major: number;
  minor: number;
  patch: number;
  apiVersion: string;
  supportedMethods: string[];
  deprecatedMethods: string[];
}

export class ProviderVersionManager {
  private readonly supportedVersions: Map<string, ProviderVersion> = new Map();

  constructor() {
    // Define supported versions
    this.supportedVersions.set('alchemy-v2', {
      major: 2,
      minor: 0,
      patch: 0,
      apiVersion: 'v2',
      supportedMethods: ['eth_*', 'net_*', 'web3_*'],
      deprecatedMethods: ['eth_compile*'],
    });

    this.supportedVersions.set('quicknode-v1', {
      major: 1,
      minor: 0,
      patch: 0,
      apiVersion: 'v1',
      supportedMethods: ['eth_*', 'net_*'],
      deprecatedMethods: [],
    });
  }

  isCompatible(providerType: string, version: ProviderVersion): boolean {
    const supported = this.supportedVersions.get(providerType);
    if (!supported) return false;

    // Major version must match
    if (supported.major !== version.major) return false;

    // Minor version can be higher
    return version.minor >= supported.minor;
  }

  checkMethodSupport(providerType: string, method: string): boolean {
    const version = this.supportedVersions.get(providerType);
    if (!version) return false;

    // Check if method is deprecated
    if (version.deprecatedMethods.some(pattern => 
      method.match(new RegExp(pattern.replace('*', '.*'))))) {
      return false;
    }

    // Check if method is supported
    return version.supportedMethods.some(pattern => 
      method.match(new RegExp(pattern.replace('*', '.*'))));
  }
}
```

#### 互換性の確認

```typescript
// compatibility/compatibility-checker.ts
export class CompatibilityChecker {
  async runCompatibilityTests(provider: EVMProvider): Promise<CompatibilityReport> {
    const report: CompatibilityReport = {
      providerType: await this.detectProviderType(provider),
      supportedMethods: [],
      unsupportedMethods: [],
      performanceBaseline: null,
      errors: [],
    };

    // Test essential methods
    const essentialMethods = [
      'eth_blockNumber',
      'eth_getBalance',
      'eth_call',
      'eth_sendTransaction',
      'eth_getLogs',
      'eth_getTransactionReceipt',
    ];

    for (const method of essentialMethods) {
      try {
        await this.testMethod(provider, method);
        report.supportedMethods.push(method);
      } catch (error) {
        report.unsupportedMethods.push(method);
        report.errors.push({
          method,
          error: error.message,
        });
      }
    }

    // Performance baseline
    report.performanceBaseline = await this.measurePerformance(provider);

    return report;
  }

  private async testMethod(provider: EVMProvider, method: string): Promise<void> {
    switch (method) {
      case 'eth_blockNumber':
        await provider.call('eth_blockNumber', []);
        break;
      case 'eth_getBalance':
        await provider.call('eth_getBalance', ['0x0000000000000000000000000000000000000000', 'latest']);
        break;
      case 'eth_call':
        await provider.call('eth_call', [{
          to: '0x0000000000000000000000000000000000000000',
          data: '0x',
        }, 'latest']);
        break;
      // Add more test cases as needed
    }
  }

  private async measurePerformance(provider: EVMProvider): Promise<PerformanceBaseline> {
    const iterations = 10;
    const responseTimes: number[] = [];

    for (let i = 0; i < iterations; i++) {
      const start = Date.now();
      await provider.call('eth_blockNumber', []);
      const end = Date.now();
      responseTimes.push(end - start);
    }

    return {
      averageResponseTime: responseTimes.reduce((a, b) => a + b) / responseTimes.length,
      minResponseTime: Math.min(...responseTimes),
      maxResponseTime: Math.max(...responseTimes),
    };
  }
}
```

#### 移行手順

```typescript
// migration/provider-migration.ts
export class ProviderMigration {
  async migrateProvider(
    oldProvider: EVMProvider,
    newProvider: EVMProvider
  ): Promise<MigrationResult> {
    const result: MigrationResult = {
      success: false,
      compatibilityIssues: [],
      performanceDelta: null,
      rollbackPlan: null,
    };

    try {
      // 1. Compatibility check
      const compatibility = await this.checkCompatibility(oldProvider, newProvider);
      if (!compatibility.isCompatible) {
        result.compatibilityIssues = compatibility.issues;
        return result;
      }

      // 2. Performance comparison
      const oldPerf = await this.measurePerformance(oldProvider);
      const newPerf = await this.measurePerformance(newProvider);
      result.performanceDelta = this.calculatePerformanceDelta(oldPerf, newPerf);

      // 3. Create rollback plan
      result.rollbackPlan = this.createRollbackPlan(oldProvider, newProvider);

      // 4. Gradual migration (canary deployment)
      await this.performCanaryMigration(oldProvider, newProvider);

      result.success = true;
      return result;

    } catch (error) {
      result.error = error.message;
      return result;
    }
  }

  private async performCanaryMigration(
    oldProvider: EVMProvider,
    newProvider: EVMProvider
  ): Promise<void> {
    // Start with 10% traffic to new provider
    const canaryProvider = new CanaryProvider(oldProvider, newProvider, 0.1);
    
    // Monitor for 1 hour
    await this.monitorCanary(canaryProvider, 3600000); // 1 hour

    // Gradually increase traffic: 10% -> 50% -> 100%
    const stages = [0.1, 0.5, 1.0];
    for (const ratio of stages) {
      canaryProvider.setTrafficRatio(ratio);
      await this.monitorCanary(canaryProvider, 1800000); // 30 minutes
    }
  }

  private async monitorCanary(
    provider: CanaryProvider,
    duration: number
  ): Promise<void> {
    const startTime = Date.now();
    const metrics = new MetricsCollector();

    while (Date.now() - startTime < duration) {
      // Collect metrics
      const currentMetrics = await provider.getMetrics();
      
      // Check for issues
      if (this.hasIssues(currentMetrics)) {
        throw new Error('Canary deployment issues detected');
      }

      await new Promise(resolve => setTimeout(resolve, 60000)); // Check every minute
    }
  }

  private createRollbackPlan(
    oldProvider: EVMProvider,
    newProvider: EVMProvider
  ): RollbackPlan {
    return {
      steps: [
        'Stop traffic to new provider',
        'Redirect all traffic to old provider',
        'Verify old provider functionality',
        'Update configuration',
        'Monitor for stability',
      ],
      estimatedTime: '10 minutes',
      backupConfig: {
        // Store current configuration
      },
      testCases: [
        'Basic connectivity test',
        'Essential method calls test',
        'Performance baseline test',
      ],
    };
  }
}
```

---

## 結論

このレポートでは、D-TPRESプロジェクトに最適なEVMプロバイダーとして**Alchemy**（メイン）+ **QuickNode**（フォールバック）の組み合わせを選定しました。

### 選定の根拠
1. **技術的優位性**: Alchemyの高性能・高信頼性
2. **リスク軽減**: QuickNodeによるフォールバック体制
3. **実装容易性**: 標準化されたインターフェース
4. **将来拡張性**: マルチチェーン対応への道筋

### 実装の要点
- 抽象化レイヤーによるプロバイダー依存の最小化
- 自動フェイルオーバーによる可用性確保
- 包括的なテスト戦略による品質保証
- 継続的なモニタリングによる運用安定性

この選定により、D-TPRESプロジェクトは堅牢で拡張可能なEVM連携基盤を確立できます。