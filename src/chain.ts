import { spawn, ChildProcess } from 'child_process';
import { ethers } from 'ethers';
import * as fs from 'fs';
import * as path from 'path';

const ANVIL_PORT = 8545;
const ANVIL_HOST = '127.0.0.1';
const ANVIL_RPC = `http://${ANVIL_HOST}:${ANVIL_PORT}`;

// Contract ABI will be loaded from compiled artifacts
const CONTRACT_ABI = [
  "function addLog(string action, string metadata) external returns (uint256)",
  "function getLog(uint256 id) external view returns (tuple(uint256 id, uint256 timestamp, address agent, string action, string metadata, bytes32 previousHash, bytes32 currentHash))",
  "function getLogCount() external view returns (uint256)",
  "function getLastHash() external view returns (bytes32)",
  "function verifyLog(uint256 id) external view returns (bool)",
  "function verifyChain(uint256 upToId) external view returns (bool)",
  "function getLogRange(uint256 start, uint256 end) external view returns (tuple(uint256 id, uint256 timestamp, address agent, string action, string metadata, bytes32 previousHash, bytes32 currentHash)[])",
  "event LogCreated(uint256 indexed id, address indexed agent, string action, bytes32 currentHash, bytes32 previousHash)"
];

export class ChainManager {
  private anvilProcess: ChildProcess | null = null;
  private provider: ethers.JsonRpcProvider | null = null;
  private wallet: ethers.Wallet | null = null;
  private contract: ethers.Contract | null = null;
  private contractAddress: string | null = null;

  /**
   * Start the Anvil local blockchain
   */
  async startAnvil(): Promise<void> {
    console.log('Starting Anvil...');

    return new Promise((resolve, reject) => {
      // Start anvil with 1s block time and state persistence
      this.anvilProcess = spawn('anvil', [
        '--block-time', '1',
        '--host', ANVIL_HOST,
        '--port', ANVIL_PORT.toString(),
        '--state-interval', '1',
        '--state', './anvil-state.json'
      ]);

      this.anvilProcess.stdout?.on('data', (data) => {
        const output = data.toString();
        console.log('[Anvil]', output);

        // Wait for Anvil to be ready
        if (output.includes('Listening on')) {
          setTimeout(resolve, 1000); // Give it a moment to be fully ready
        }
      });

      this.anvilProcess.stderr?.on('data', (data) => {
        console.error('[Anvil Error]', data.toString());
      });

      this.anvilProcess.on('error', (error) => {
        console.error('Failed to start Anvil:', error);
        reject(error);
      });

      this.anvilProcess.on('exit', (code) => {
        console.log(`Anvil exited with code ${code}`);
      });

      // Timeout if Anvil doesn't start
      setTimeout(() => {
        if (!this.provider) {
          reject(new Error('Anvil startup timeout'));
        }
      }, 10000);
    });
  }

  /**
   * Initialize the provider and wallet
   */
  async initProvider(): Promise<void> {
    console.log('Initializing provider...');
    this.provider = new ethers.JsonRpcProvider(ANVIL_RPC);

    // Use the first default Anvil account
    const privateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';
    this.wallet = new ethers.Wallet(privateKey, this.provider);

    console.log('Wallet address:', this.wallet.address);
  }

  /**
   * Deploy the AgentLog contract
   */
  async deployContract(): Promise<string> {
    if (!this.wallet) {
      throw new Error('Wallet not initialized');
    }

    console.log('Deploying AgentLog contract...');

    // Load compiled contract bytecode
    const contractPath = path.join(process.cwd(), 'contracts', 'AgentLog.sol');

    // For simplicity, using a pre-compiled bytecode
    // In production, you'd compile with solc or foundry
    const factory = new ethers.ContractFactory(
      CONTRACT_ABI,
      this.getContractBytecode(),
      this.wallet
    );

    const contract = await factory.deploy();
    await contract.waitForDeployment();

    this.contractAddress = await contract.getAddress();
    this.contract = contract;

    console.log('Contract deployed at:', this.contractAddress);

    // Save contract address for persistence
    this.saveContractAddress(this.contractAddress);

    return this.contractAddress;
  }

  /**
   * Load existing contract
   */
  async loadContract(address: string): Promise<void> {
    if (!this.wallet) {
      throw new Error('Wallet not initialized');
    }

    console.log('Loading contract at:', address);
    this.contractAddress = address;
    this.contract = new ethers.Contract(address, CONTRACT_ABI, this.wallet);
  }

  /**
   * Add a log entry to the blockchain
   */
  async addLog(action: string, metadata: string): Promise<{ id: bigint; hash: string; txHash: string }> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    const tx = await this.contract.addLog(action, metadata);
    const receipt = await tx.wait();

    // Parse the LogCreated event
    const event = receipt.logs
      .map((log: any) => {
        try {
          return this.contract!.interface.parseLog(log);
        } catch {
          return null;
        }
      })
      .find((e: any) => e && e.name === 'LogCreated');

    if (!event) {
      throw new Error('LogCreated event not found');
    }

    return {
      id: event.args.id,
      hash: event.args.currentHash,
      txHash: receipt.hash
    };
  }

  /**
   * Get a log entry by ID
   */
  async getLog(id: number): Promise<any> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    return await this.contract.getLog(id);
  }

  /**
   * Get total log count
   */
  async getLogCount(): Promise<bigint> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    return await this.contract.getLogCount();
  }

  /**
   * Get logs in a range
   */
  async getLogRange(start: number, end: number): Promise<any[]> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    return await this.contract.getLogRange(start, end);
  }

  /**
   * Verify a specific log entry
   */
  async verifyLog(id: number): Promise<boolean> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    return await this.contract.verifyLog(id);
  }

  /**
   * Verify the entire chain
   */
  async verifyChain(upToId: number = 0): Promise<boolean> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    return await this.contract.verifyChain(upToId);
  }

  /**
   * Stop Anvil
   */
  async stop(): Promise<void> {
    if (this.anvilProcess) {
      console.log('Stopping Anvil...');
      this.anvilProcess.kill();
      this.anvilProcess = null;
    }
  }

  /**
   * Get contract address
   */
  getContractAddress(): string | null {
    return this.contractAddress;
  }

  /**
   * Helper to save contract address to file
   */
  private saveContractAddress(address: string): void {
    const configPath = path.join(process.cwd(), '.safeclaw-config.json');
    const config = { contractAddress: address };
    fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
  }

  /**
   * Helper to load contract address from file
   */
  static loadContractAddress(): string | null {
    const configPath = path.join(process.cwd(), '.safeclaw-config.json');
    try {
      const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
      return config.contractAddress;
    } catch {
      return null;
    }
  }

  /**
   * Get contract bytecode
   * In production, this would be loaded from compiled artifacts
   */
  private getContractBytecode(): string {
    // This is a placeholder - in production, compile with solc/foundry
    // For now, we'll need to compile the contract separately
    throw new Error('Contract must be compiled first. Run: forge build');
  }
}
