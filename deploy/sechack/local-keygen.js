#!/usr/bin/env node

/**
 * local-keygen.js - ローカル鍵生成とkFrag作成スクリプト
 *
 * O-Browser（ローカル）でのPhase 1処理をシミュレート：
 * 1. 秘密データの準備
 * 2. ローカル暗号化処理の実行
 * 3. 結果の保存とAO Processへの送信準備
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

/**
 * MVPデモ設定
 */
const CONFIG = {
    secret: "This is a secret message for D-TPRES MVP demo",
    threshold: 2,
    totalShares: 3,
    outputDir: path.join(__dirname, 'output'),
    resultFile: 'local-keygen-result.json'
};

/**
 * シミュレートされたローカル暗号化処理
 * 実際の実装では Rust WASM モジュールを呼び出す
 */
class LocalOwnerProcessor {
    constructor() {
        this.processId = `local-${Date.now()}`;
    }

    /**
     * 完全なローカル処理フローをシミュレート
     */
    async processOwnerLocal(params) {
        console.log("🔐 Starting local owner processing...");

        // Step 1: Owner鍵ペア生成
        console.log("1. Generating owner keypair...");
        const ownerKeys = this.generateKeypair();

        // Step 2: Accessor鍵ペア生成（MVP用）
        console.log("2. Generating accessor keypair...");
        const accessorKeys = this.generateKeypair();

        // Step 3: Shamir秘密分散
        console.log("3. Splitting secret using Shamir's Secret Sharing...");
        const shares = this.splitSecret(params.secret, params.threshold, params.totalShares);

        // Step 4: 対称鍵生成
        console.log("4. Generating symmetric key...");
        const symmetricKey = this.generateSymmetricKey();

        // Step 5: シェア暗号化
        console.log("5. Encrypting shares...");
        const encryptedShares = this.encryptShares(shares, symmetricKey);

        // Step 6: カプセル生成
        console.log("6. Creating PRE capsule...");
        const capsule = this.createCapsule(ownerKeys.publicKey, symmetricKey);

        // Step 7: kFrag生成
        console.log("7. Generating kFrags...");
        const kfrags = this.generateKFrags(
            ownerKeys.secretKey,
            accessorKeys.publicKey,
            params.threshold,
            params.totalShares
        );

        console.log("✅ Local processing completed successfully!");

        return {
            processId: this.processId,
            ownerKeys,
            accessorKeys,
            capsule,
            encryptedShares,
            kfrags,
            symmetricKeyHash: this.hashKey(symmetricKey),
            timestamp: Date.now()
        };
    }

    /**
     * 鍵ペア生成をシミュレート
     */
    generateKeypair() {
        // MVP: シミュレートされた鍵ペア
        const secretKey = crypto.randomBytes(32);
        const publicKey = crypto.randomBytes(33); // 圧縮公開鍵サイズ

        return {
            secretKey: Array.from(secretKey),
            publicKey: Array.from(publicKey)
        };
    }

    /**
     * Shamir秘密分散をシミュレート
     */
    splitSecret(secret, threshold, totalShares) {
        const secretBytes = Buffer.from(secret, 'utf8');
        const shares = [];

        for (let i = 0; i < totalShares; i++) {
            // MVP: 簡単なXOR分散（実際の実装ではShamir実装を使用）
            const shareData = secretBytes.map((byte, idx) =>
                byte ^ ((i + 1) * (idx + 1)) % 256
            );

            shares.push({
                index: i + 1,
                data: Array.from(shareData)
            });
        }

        return shares;
    }

    /**
     * 対称鍵生成
     */
    generateSymmetricKey() {
        return Array.from(crypto.randomBytes(32)); // AES-256
    }

    /**
     * シェア暗号化
     */
    encryptShares(shares, symmetricKey) {
        return shares.map(share => {
            // MVP: 簡単なXOR暗号化（実際の実装ではAES-GCMを使用）
            const nonce = Array.from(crypto.randomBytes(12));
            const encryptedData = share.data.map((byte, idx) =>
                byte ^ symmetricKey[idx % symmetricKey.length]
            );
            const authTag = Array.from(crypto.randomBytes(16));

            return {
                index: share.index,
                encryptedData,
                authTag,
                nonce
            };
        });
    }

    /**
     * カプセル生成をシミュレート
     */
    createCapsule(publicKey, symmetricKey) {
        // MVP: シミュレートされたカプセル
        const capsuleData = [
            ...publicKey.slice(0, 16),
            ...symmetricKey.slice(0, 16),
            ...Array.from(crypto.randomBytes(32))
        ];

        return {
            data: capsuleData
        };
    }

    /**
     * kFrag生成をシミュレート
     */
    generateKFrags(ownerSecretKey, accessorPublicKey, threshold, totalShares) {
        const kfrags = [];

        for (let i = 0; i < totalShares; i++) {
            // MVP: シミュレートされたkFrag
            const keyData = [
                ...ownerSecretKey.slice(0, 16),
                ...accessorPublicKey.slice(0, 16),
                i, threshold, totalShares,
                ...Array.from(crypto.randomBytes(32))
            ];

            const verificationData = [
                ...accessorPublicKey.slice(0, 16),
                ...Array.from(crypto.randomBytes(16))
            ];

            kfrags.push({
                id: i,
                keyData,
                verificationData,
                precursor: []
            });
        }

        return kfrags;
    }

    /**
     * 鍵のハッシュ生成
     */
    hashKey(key) {
        const hash = crypto.createHash('sha256');
        hash.update(Buffer.from(key));
        return Array.from(hash.digest());
    }
}

/**
 * ファイル操作ユーティリティ
 */
class FileManager {
    constructor(outputDir) {
        this.outputDir = outputDir;
        this.ensureOutputDir();
    }

    ensureOutputDir() {
        if (!fs.existsSync(this.outputDir)) {
            fs.mkdirSync(this.outputDir, { recursive: true });
            console.log(`📁 Created output directory: ${this.outputDir}`);
        }
    }

    saveResult(result, filename) {
        const filePath = path.join(this.outputDir, filename);
        const jsonData = JSON.stringify(result, null, 2);

        fs.writeFileSync(filePath, jsonData, 'utf8');
        console.log(`💾 Saved result to: ${filePath}`);
        return filePath;
    }

    loadResult(filename) {
        const filePath = path.join(this.outputDir, filename);

        if (!fs.existsSync(filePath)) {
            throw new Error(`Result file not found: ${filePath}`);
        }

        const jsonData = fs.readFileSync(filePath, 'utf8');
        return JSON.parse(jsonData);
    }
}

/**
 * AOメッセージ準備ユーティリティ
 */
class AOMessagePreparer {
    static prepareOwnerMessage(result) {
        return {
            action: "transfer-kfrags",
            data: {
                kfrags: result.kfrags.map(kf => ({
                    id: kf.id,
                    key_data: kf.keyData,
                    verification_data: kf.verificationData,
                    precursor: kf.precursor
                })),
                capsule: {
                    data: result.capsule.data
                },
                holder_process_id: "holder_process_mvp"
            }
        };
    }

    static prepareHolderMessage(result) {
        const kfrag = result.kfrags[0]; // MVP: 最初のkFragのみ使用

        return {
            action: "store-kfrag",
            data: {
                kfrag: {
                    id: kfrag.id,
                    key_data: kfrag.keyData,
                    verification_data: kfrag.verificationData,
                    precursor: kfrag.precursor
                },
                capsule: {
                    data: result.capsule.data
                }
            }
        };
    }
}

/**
 * メイン実行関数
 */
async function main() {
    console.log("🚀 D-TPRES MVP Local Key Generation Demo");
    console.log("=====================================\n");

    try {
        // ローカル処理実行
        const processor = new LocalOwnerProcessor();
        const result = await processor.processOwnerLocal(CONFIG);

        // ファイル管理
        const fileManager = new FileManager(CONFIG.outputDir);
        const resultFile = fileManager.saveResult(result, CONFIG.resultFile);

        // AOメッセージ準備
        const ownerMessage = AOMessagePreparer.prepareOwnerMessage(result);
        const holderMessage = AOMessagePreparer.prepareHolderMessage(result);

        fileManager.saveResult(ownerMessage, 'owner-message.json');
        fileManager.saveResult(holderMessage, 'holder-message.json');

        // 結果サマリー表示
        console.log("\n📊 Processing Summary:");
        console.log(`   Process ID: ${result.processId}`);
        console.log(`   Secret length: ${CONFIG.secret.length} chars`);
        console.log(`   Threshold: ${CONFIG.threshold}/${CONFIG.totalShares}`);
        console.log(`   Encrypted shares: ${result.encryptedShares.length}`);
        console.log(`   kFrags generated: ${result.kfrags.length}`);
        console.log(`   Capsule size: ${result.capsule.data.length} bytes`);

        console.log("\n📁 Generated files:");
        console.log(`   Results: ${resultFile}`);
        console.log(`   Owner message: ${path.join(CONFIG.outputDir, 'owner-message.json')}`);
        console.log(`   Holder message: ${path.join(CONFIG.outputDir, 'holder-message.json')}`);

        console.log("\n🎯 Next steps:");
        console.log("   1. Run 'node spawn-processes.js' to create AO processes");
        console.log("   2. Run 'node test-flow.js' to test the complete flow");

        console.log("\n✅ Local key generation completed successfully!");

    } catch (error) {
        console.error("❌ Error during local key generation:", error.message);
        process.exit(1);
    }
}

// CLIからの実行時のメイン処理
if (require.main === module) {
    main().catch(error => {
        console.error("Fatal error:", error);
        process.exit(1);
    });
}

// モジュールとしての公開
module.exports = {
    LocalOwnerProcessor,
    FileManager,
    AOMessagePreparer,
    CONFIG
};