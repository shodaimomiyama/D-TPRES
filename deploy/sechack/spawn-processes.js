#!/usr/bin/env node

/**
 * spawn-processes.js - AO Process Spawn スクリプト
 *
 * Phase 2: AO Network上でのプロセス管理
 * 1. Owner-Processのspawn
 * 2. Holder-Processのspawn
 * 3. Process IDの管理と保存
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

/**
 * MVP設定
 */
const CONFIG = {
    outputDir: path.join(__dirname, 'output'),
    processesFile: 'spawned-processes.json',
    wasmModule: '../target/wasm32-unknown-unknown/release/d_tpres.wasm',
    aoConfig: {
        // AO Network設定（MVP用ダミー）
        gatewayUrl: 'https://ao-gateway.arweave.net',
        walletPath: './wallet.json'
    }
};

/**
 * AO Process管理クラス
 */
class AOProcessManager {
    constructor() {
        this.processes = {
            owner: null,
            holders: []
        };
        this.fileManager = new FileManager(CONFIG.outputDir);
    }

    /**
     * Owner-Processをspawn
     */
    async spawnOwnerProcess() {
        console.log("🚀 Spawning Owner-Process...");

        // MVP: シミュレートされたProcess spawn
        const processId = this.generateProcessId('owner');
        const process = {
            id: processId,
            type: 'owner',
            status: 'active',
            tags: {
                'Data-Protocol': 'DTPRES',
                'Type': 'Owner-Process',
                'Version': '0.1.0-mvp'
            },
            spawnedAt: Date.now(),
            lastActivity: Date.now()
        };

        this.processes.owner = process;
        console.log(`✅ Owner-Process spawned: ${processId}`);

        return process;
    }

    /**
     * Holder-Processをspawn（MVP: 1つのみ）
     */
    async spawnHolderProcess(index = 0) {
        console.log(`🚀 Spawning Holder-Process ${index}...`);

        const processId = this.generateProcessId('holder', index);
        const process = {
            id: processId,
            type: 'holder',
            index: index,
            status: 'active',
            tags: {
                'Data-Protocol': 'DTPRES',
                'Type': 'Holder-Process',
                'Index': index.toString(),
                'Version': '0.1.0-mvp'
            },
            spawnedAt: Date.now(),
            lastActivity: Date.now(),
            storedKFrags: []
        };

        this.processes.holders.push(process);
        console.log(`✅ Holder-Process ${index} spawned: ${processId}`);

        return process;
    }

    /**
     * Requester-Processをspawn（将来用）
     */
    async spawnRequesterProcess() {
        console.log("🚀 Spawning Requester-Process...");

        const processId = this.generateProcessId('requester');
        const process = {
            id: processId,
            type: 'requester',
            status: 'active',
            tags: {
                'Data-Protocol': 'DTPRES',
                'Type': 'Requester-Process',
                'Version': '0.1.0-mvp'
            },
            spawnedAt: Date.now(),
            lastActivity: Date.now()
        };

        console.log(`✅ Requester-Process spawned: ${processId}`);
        return process;
    }

    /**
     * すべてのプロセスを一括spawn
     */
    async spawnAllProcesses() {
        console.log("🔄 Spawning all D-TPRES processes...");

        try {
            // Owner-Process spawn
            await this.spawnOwnerProcess();

            // Holder-Process spawn（MVP: 1つのみ）
            await this.spawnHolderProcess(0);

            console.log("✅ All processes spawned successfully!");
            return this.processes;

        } catch (error) {
            console.error("❌ Error spawning processes:", error.message);
            throw error;
        }
    }

    /**
     * プロセス状態の保存
     */
    saveProcesses() {
        const processData = {
            metadata: {
                createdAt: Date.now(),
                version: '0.1.0-mvp',
                description: 'D-TPRES MVP Process Registry'
            },
            processes: this.processes
        };

        return this.fileManager.saveResult(processData, CONFIG.processesFile);
    }

    /**
     * プロセス状態の読み込み
     */
    loadProcesses() {
        try {
            const data = this.fileManager.loadResult(CONFIG.processesFile);
            this.processes = data.processes;
            return data;
        } catch (error) {
            console.log("No existing processes found, starting fresh.");
            return null;
        }
    }

    /**
     * プロセスIDの生成
     */
    generateProcessId(type, index = null) {
        const timestamp = Date.now();
        const random = crypto.randomBytes(4).toString('hex');
        const indexSuffix = index !== null ? `-${index}` : '';
        return `dtpres-${type}${indexSuffix}-${timestamp}-${random}`;
    }

    /**
     * プロセス状況の表示
     */
    displayProcessStatus() {
        console.log("\n📊 Process Status Summary:");
        console.log("========================");

        if (this.processes.owner) {
            console.log(`🔑 Owner-Process: ${this.processes.owner.id}`);
            console.log(`    Status: ${this.processes.owner.status}`);
            console.log(`    Spawned: ${new Date(this.processes.owner.spawnedAt).toISOString()}`);
        }

        if (this.processes.holders.length > 0) {
            console.log(`🔒 Holder-Processes: ${this.processes.holders.length}`);
            this.processes.holders.forEach((holder, idx) => {
                console.log(`    [${idx}] ${holder.id}`);
                console.log(`        Status: ${holder.status}`);
                console.log(`        kFrags: ${holder.storedKFrags.length}`);
            });
        }

        console.log("========================\n");
    }
}

/**
 * ファイル管理クラス
 */
class FileManager {
    constructor(outputDir) {
        this.outputDir = outputDir;
        this.ensureOutputDir();
    }

    ensureOutputDir() {
        if (!fs.existsSync(this.outputDir)) {
            fs.mkdirSync(this.outputDir, { recursive: true });
        }
    }

    saveResult(result, filename) {
        const filePath = path.join(this.outputDir, filename);
        const jsonData = JSON.stringify(result, null, 2);

        fs.writeFileSync(filePath, jsonData, 'utf8');
        console.log(`💾 Saved to: ${filePath}`);
        return filePath;
    }

    loadResult(filename) {
        const filePath = path.join(this.outputDir, filename);

        if (!fs.existsSync(filePath)) {
            throw new Error(`File not found: ${filePath}`);
        }

        const jsonData = fs.readFileSync(filePath, 'utf8');
        return JSON.parse(jsonData);
    }
}

/**
 * WASM デプロイ管理（MVP用シミュレーション）
 */
class WasmDeployManager {
    constructor() {
        this.wasmPath = CONFIG.wasmModule;
    }

    /**
     * WASMモジュールの存在確認
     */
    checkWasmModule() {
        const fullPath = path.resolve(__dirname, this.wasmPath);

        if (!fs.existsSync(fullPath)) {
            console.log(`⚠️  WASM module not found at: ${fullPath}`);
            console.log("   Building WASM module is not required for MVP demo.");
            console.log("   Processes will be spawned with simulated functionality.");
            return false;
        }

        console.log(`✅ WASM module found: ${fullPath}`);
        return true;
    }

    /**
     * WASMモジュールの情報を取得
     */
    getWasmInfo() {
        const fullPath = path.resolve(__dirname, this.wasmPath);

        if (fs.existsSync(fullPath)) {
            const stats = fs.statSync(fullPath);
            return {
                path: fullPath,
                size: stats.size,
                modified: stats.mtime
            };
        }

        return null;
    }
}

/**
 * メイン実行関数
 */
async function main() {
    console.log("🚀 D-TPRES MVP Process Spawning Demo");
    console.log("===================================\n");

    try {
        // WASM モジュール確認
        const wasmManager = new WasmDeployManager();
        const wasmExists = wasmManager.checkWasmModule();

        if (wasmExists) {
            const wasmInfo = wasmManager.getWasmInfo();
            console.log(`📦 WASM module: ${wasmInfo.size} bytes`);
            console.log(`📅 Modified: ${wasmInfo.modified.toISOString()}\n`);
        }

        // プロセス管理開始
        const processManager = new AOProcessManager();

        // 既存プロセスのチェック
        console.log("🔍 Checking for existing processes...");
        const existingProcesses = processManager.loadProcesses();

        if (existingProcesses && existingProcesses.processes.owner) {
            console.log("⚠️  Existing processes found!");
            console.log("   Use --force to respawn or --status to view current state.");

            if (process.argv.includes('--status')) {
                processManager.displayProcessStatus();
                return;
            }

            if (!process.argv.includes('--force')) {
                console.log("   Exiting without changes.");
                return;
            }

            console.log("🔄 Force respawning processes...");
        }

        // プロセスのspawn
        await processManager.spawnAllProcesses();

        // 結果の保存
        const savedPath = processManager.saveProcesses();

        // 状況表示
        processManager.displayProcessStatus();

        console.log("📁 Generated files:");
        console.log(`   Process registry: ${savedPath}`);

        console.log("\n🎯 Next steps:");
        console.log("   1. Verify processes with 'node spawn-processes.js --status'");
        console.log("   2. Run 'node test-flow.js' to test the complete flow");

        console.log("\n✅ Process spawning completed successfully!");

    } catch (error) {
        console.error("❌ Error during process spawning:", error.message);
        process.exit(1);
    }
}

/**
 * CLI引数処理
 */
function handleCliArgs() {
    const args = process.argv.slice(2);

    if (args.includes('--help') || args.includes('-h')) {
        console.log("D-TPRES Process Spawning Tool");
        console.log("");
        console.log("Usage: node spawn-processes.js [options]");
        console.log("");
        console.log("Options:");
        console.log("  --force    Force respawn existing processes");
        console.log("  --status   Show current process status");
        console.log("  --help     Show this help message");
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
    AOProcessManager,
    WasmDeployManager,
    FileManager,
    CONFIG
};