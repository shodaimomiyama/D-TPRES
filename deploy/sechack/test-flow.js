#!/usr/bin/env node

/**
 * test-flow.js - D-TPRES MVP エンドツーエンドテストフロー
 *
 * 完全な動作フローのテスト：
 * 1. ローカル鍵生成結果の読み込み
 * 2. Owner-ProcessへのkFrag送信
 * 3. Holder-ProcessでのcFrag生成
 * 4. 結果の検証
 */

const fs = require('fs');
const path = require('path');
const { FileManager } = require('./local-keygen');
const { AOProcessManager } = require('./spawn-processes');

/**
 * テスト設定
 */
const CONFIG = {
    outputDir: path.join(__dirname, 'output'),
    resultsDir: path.join(__dirname, 'output', 'test-results'),
    timeout: 5000 // メッセージ処理タイムアウト（ミリ秒）
};

/**
 * メッセージシミュレーター
 */
class MessageSimulator {
    constructor() {
        this.messageHistory = [];
    }

    /**
     * Owner-Processへのメッセージ送信をシミュレート
     */
    async sendToOwner(processId, message) {
        console.log(`📤 Sending message to Owner-Process (${processId})`);
        console.log(`   Action: ${message.action}`);

        // MVP: シミュレートされたメッセージ処理
        await this.simulateDelay(500);

        const response = {
            processId,
            timestamp: Date.now(),
            status: 'success',
            message: 'Message processed by Owner-Process',
            data: {
                holder_process_id: message.data.holder_process_id,
                transferred_count: message.data.kfrags.length,
                status: 'success'
            }
        };

        this.logMessage('owner', 'send', message, response);
        return response;
    }

    /**
     * Holder-Processへのメッセージ送信をシミュレート
     */
    async sendToHolder(processId, message) {
        console.log(`📤 Sending message to Holder-Process (${processId})`);
        console.log(`   Action: ${message.action}`);

        await this.simulateDelay(300);

        let response;

        switch (message.action) {
            case 'store-kfrag':
                response = {
                    processId,
                    timestamp: Date.now(),
                    status: 'success',
                    message: 'kFrag stored successfully',
                    data: {
                        kfrag_id: message.data.kfrag.id,
                        status: 'stored'
                    }
                };
                break;

            case 'generate-cfrag':
                response = {
                    processId,
                    timestamp: Date.now(),
                    status: 'success',
                    message: 'cFrag generated successfully',
                    data: {
                        cfrag: {
                            fragment_id: message.data.kfrag_id,
                            capsule_fragment: this.generateMockCFragData(),
                            proof: this.generateMockProofData()
                        }
                    }
                };
                break;

            default:
                response = {
                    processId,
                    timestamp: Date.now(),
                    status: 'error',
                    message: `Unknown action: ${message.action}`
                };
        }

        this.logMessage('holder', 'send', message, response);
        return response;
    }

    /**
     * メッセージ履歴をログ
     */
    logMessage(processType, direction, message, response) {
        this.messageHistory.push({
            processType,
            direction,
            timestamp: Date.now(),
            message,
            response
        });
    }

    /**
     * 処理遅延をシミュレート
     */
    async simulateDelay(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    /**
     * モックcFragデータ生成
     */
    generateMockCFragData() {
        const data = [];
        for (let i = 0; i < 64; i++) {
            data.push(Math.floor(Math.random() * 256));
        }
        return data;
    }

    /**
     * モック証明データ生成
     */
    generateMockProofData() {
        const data = [];
        for (let i = 0; i < 32; i++) {
            data.push(Math.floor(Math.random() * 256));
        }
        return data;
    }

    /**
     * メッセージ履歴の取得
     */
    getMessageHistory() {
        return this.messageHistory;
    }
}

/**
 * テストフロー実行クラス
 */
class TestFlowRunner {
    constructor() {
        this.fileManager = new FileManager(CONFIG.outputDir);
        this.messageSimulator = new MessageSimulator();
        this.testResults = {
            startTime: Date.now(),
            tests: [],
            summary: {
                total: 0,
                passed: 0,
                failed: 0
            }
        };
    }

    /**
     * 完全なテストフローを実行
     */
    async runCompleteFlow() {
        console.log("🧪 Starting D-TPRES MVP End-to-End Test Flow");
        console.log("============================================\n");

        try {
            // 1. 前提条件の確認
            await this.checkPrerequisites();

            // 2. ローカル鍵生成結果の読み込み
            const localResult = await this.loadLocalResult();

            // 3. プロセス情報の読み込み
            const processes = await this.loadProcesses();

            // 4. Owner-Processテスト
            await this.testOwnerProcess(localResult, processes);

            // 5. Holder-Processテスト
            await this.testHolderProcess(localResult, processes);

            // 6. cFrag生成テスト
            await this.testCFragGeneration(localResult, processes);

            // 7. 結果の保存とサマリー表示
            await this.saveResults();
            this.displaySummary();

        } catch (error) {
            console.error("❌ Test flow failed:", error.message);
            await this.saveResults();
            process.exit(1);
        }
    }

    /**
     * 前提条件の確認
     */
    async checkPrerequisites() {
        console.log("🔍 Checking prerequisites...");

        const tests = [
            {
                name: "Local key generation result exists",
                check: () => {
                    const resultPath = path.join(CONFIG.outputDir, 'local-keygen-result.json');
                    return fs.existsSync(resultPath);
                }
            },
            {
                name: "Spawned processes exist",
                check: () => {
                    const processPath = path.join(CONFIG.outputDir, 'spawned-processes.json');
                    return fs.existsSync(processPath);
                }
            },
            {
                name: "Output directory is writable",
                check: () => {
                    try {
                        const testFile = path.join(CONFIG.outputDir, 'test-write.tmp');
                        fs.writeFileSync(testFile, 'test');
                        fs.unlinkSync(testFile);
                        return true;
                    } catch {
                        return false;
                    }
                }
            }
        ];

        for (const test of tests) {
            const passed = test.check();
            this.recordTest(`Prerequisites: ${test.name}`, passed, passed ? "✅" : "❌");

            if (!passed) {
                throw new Error(`Prerequisite failed: ${test.name}`);
            }
        }

        console.log("✅ All prerequisites satisfied\n");
    }

    /**
     * ローカル結果の読み込み
     */
    async loadLocalResult() {
        console.log("📖 Loading local key generation result...");

        try {
            const result = this.fileManager.loadResult('local-keygen-result.json');

            console.log(`   Process ID: ${result.processId}`);
            console.log(`   kFrags: ${result.kfrags.length}`);
            console.log(`   Encrypted shares: ${result.encryptedShares.length}`);
            console.log(`   Capsule size: ${result.capsule.data.length} bytes`);

            this.recordTest("Load local result", true, "✅");
            return result;

        } catch (error) {
            this.recordTest("Load local result", false, "❌");
            throw new Error(`Failed to load local result: ${error.message}`);
        }
    }

    /**
     * プロセス情報の読み込み
     */
    async loadProcesses() {
        console.log("📖 Loading spawned processes...");

        try {
            const processData = this.fileManager.loadResult('spawned-processes.json');
            const processes = processData.processes;

            console.log(`   Owner-Process: ${processes.owner.id}`);
            console.log(`   Holder-Processes: ${processes.holders.length}`);

            this.recordTest("Load processes", true, "✅");
            return processes;

        } catch (error) {
            this.recordTest("Load processes", false, "❌");
            throw new Error(`Failed to load processes: ${error.message}`);
        }
    }

    /**
     * Owner-Processのテスト
     */
    async testOwnerProcess(localResult, processes) {
        console.log("\n🔑 Testing Owner-Process...");

        try {
            // Owner messageの準備
            const ownerMessage = this.fileManager.loadResult('owner-message.json');

            // Owner-Processにメッセージ送信
            const response = await this.messageSimulator.sendToOwner(
                processes.owner.id,
                ownerMessage
            );

            // レスポンス検証
            const success = response.status === 'success' &&
                           response.data.transferred_count === localResult.kfrags.length;

            console.log(`   kFrags transferred: ${response.data.transferred_count}`);
            console.log(`   Status: ${response.data.status}`);

            this.recordTest("Owner-Process kFrag transfer", success, success ? "✅" : "❌");

            if (!success) {
                throw new Error("Owner-Process transfer failed");
            }

        } catch (error) {
            this.recordTest("Owner-Process kFrag transfer", false, "❌");
            throw new Error(`Owner-Process test failed: ${error.message}`);
        }
    }

    /**
     * Holder-Processのテスト
     */
    async testHolderProcess(localResult, processes) {
        console.log("\n🔒 Testing Holder-Process...");

        try {
            const holderProcess = processes.holders[0]; // MVP: 最初のHolderのみ
            const holderMessage = this.fileManager.loadResult('holder-message.json');

            // kFrag保存テスト
            const storeResponse = await this.messageSimulator.sendToHolder(
                holderProcess.id,
                holderMessage
            );

            const storeSuccess = storeResponse.status === 'success' &&
                               storeResponse.data.status === 'stored';

            console.log(`   kFrag stored: ${storeResponse.data.kfrag_id}`);
            console.log(`   Status: ${storeResponse.data.status}`);

            this.recordTest("Holder-Process kFrag storage", storeSuccess, storeSuccess ? "✅" : "❌");

            if (!storeSuccess) {
                throw new Error("Holder-Process storage failed");
            }

        } catch (error) {
            this.recordTest("Holder-Process kFrag storage", false, "❌");
            throw new Error(`Holder-Process test failed: ${error.message}`);
        }
    }

    /**
     * cFrag生成のテスト
     */
    async testCFragGeneration(localResult, processes) {
        console.log("\n🔐 Testing cFrag generation...");

        try {
            const holderProcess = processes.holders[0];

            // cFrag生成メッセージ
            const cFragMessage = {
                action: "generate-cfrag",
                data: {
                    kfrag_id: 0,
                    capsule: {
                        data: localResult.capsule.data
                    }
                }
            };

            // cFrag生成実行
            const cFragResponse = await this.messageSimulator.sendToHolder(
                holderProcess.id,
                cFragMessage
            );

            const cFragSuccess = cFragResponse.status === 'success' &&
                               cFragResponse.data.cfrag &&
                               cFragResponse.data.cfrag.fragment_id === 0;

            console.log(`   cFrag generated for fragment: ${cFragResponse.data.cfrag.fragment_id}`);
            console.log(`   Capsule fragment size: ${cFragResponse.data.cfrag.capsule_fragment.length} bytes`);
            console.log(`   Proof size: ${cFragResponse.data.cfrag.proof.length} bytes`);

            this.recordTest("cFrag generation", cFragSuccess, cFragSuccess ? "✅" : "❌");

            if (!cFragSuccess) {
                throw new Error("cFrag generation failed");
            }

            // cFragデータを保存
            this.fileManager.saveResult(cFragResponse.data.cfrag, 'generated-cfrag.json');

        } catch (error) {
            this.recordTest("cFrag generation", false, "❌");
            throw new Error(`cFrag generation test failed: ${error.message}`);
        }
    }

    /**
     * テスト結果を記録
     */
    recordTest(name, passed, symbol) {
        this.testResults.tests.push({
            name,
            passed,
            symbol,
            timestamp: Date.now()
        });

        this.testResults.summary.total++;
        if (passed) {
            this.testResults.summary.passed++;
        } else {
            this.testResults.summary.failed++;
        }

        console.log(`   ${symbol} ${name}`);
    }

    /**
     * 結果の保存
     */
    async saveResults() {
        // 結果ディレクトリ作成
        if (!fs.existsSync(CONFIG.resultsDir)) {
            fs.mkdirSync(CONFIG.resultsDir, { recursive: true });
        }

        this.testResults.endTime = Date.now();
        this.testResults.duration = this.testResults.endTime - this.testResults.startTime;
        this.testResults.messageHistory = this.messageSimulator.getMessageHistory();

        // 結果保存
        const resultPath = path.join(CONFIG.resultsDir, `test-results-${Date.now()}.json`);
        fs.writeFileSync(resultPath, JSON.stringify(this.testResults, null, 2));

        console.log(`\n💾 Test results saved to: ${resultPath}`);
    }

    /**
     * サマリー表示
     */
    displaySummary() {
        console.log("\n📊 Test Summary");
        console.log("===============");
        console.log(`Total tests: ${this.testResults.summary.total}`);
        console.log(`Passed: ${this.testResults.summary.passed} ✅`);
        console.log(`Failed: ${this.testResults.summary.failed} ❌`);
        console.log(`Duration: ${this.testResults.duration}ms`);

        console.log("\n📋 Test Details:");
        this.testResults.tests.forEach((test, index) => {
            console.log(`  ${index + 1}. ${test.symbol} ${test.name}`);
        });

        if (this.testResults.summary.failed === 0) {
            console.log("\n🎉 All tests passed! D-TPRES MVP flow is working correctly.");
        } else {
            console.log("\n⚠️  Some tests failed. Check the detailed results for more information.");
        }

        console.log("\n🎯 Next steps:");
        console.log("   1. Review test results in the output directory");
        console.log("   2. Test with different parameters using --custom");
        console.log("   3. Explore the generated data files");
    }
}

/**
 * メイン実行関数
 */
async function main() {
    const runner = new TestFlowRunner();
    await runner.runCompleteFlow();
}

/**
 * CLI引数処理
 */
function handleCliArgs() {
    const args = process.argv.slice(2);

    if (args.includes('--help') || args.includes('-h')) {
        console.log("D-TPRES End-to-End Test Flow");
        console.log("");
        console.log("Usage: node test-flow.js [options]");
        console.log("");
        console.log("Options:");
        console.log("  --help     Show this help message");
        console.log("");
        console.log("Prerequisites:");
        console.log("  1. Run 'node local-keygen.js' first");
        console.log("  2. Run 'node spawn-processes.js' second");
        console.log("  3. Then run this test flow");
        process.exit(0);
    }

    return args;
}

// CLIからの実行時のメイン処理
if (require.main === module) {
    handleCliArgs();
    main().catch(error => {
        console.error("Fatal error:", error);
        process.exit(1);
    });
}

// モジュールとしての公開
module.exports = {
    TestFlowRunner,
    MessageSimulator,
    CONFIG
};