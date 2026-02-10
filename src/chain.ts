import { spawn, ChildProcess } from 'child_process';
import { ethers } from 'ethers';
import * as fs from 'fs';
import * as path from 'path';
import * as dotenv from 'dotenv';

// Load environment variables
dotenv.config();

const ETHEREUM_RPC_URL = process.env.ETHEREUM_RPC_URL || 'http://127.0.0.1:8545';

// Load contract ABI from compiled artifacts
function loadContractABI(): any[] {
  try {
    const artifactPath = path.join(process.cwd(), 'out', 'AgentLog.sol', 'AgentLog.json');
    const artifact = JSON.parse(fs.readFileSync(artifactPath, 'utf-8'));
    return artifact.abi;
  } catch (error) {
    console.error('Failed to load contract ABI from compiled artifacts:', error);
    throw new Error('Contract artifacts not found. Please run: forge build');
  }
}

const CONTRACT_ABI = loadContractABI();

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

    // Parse host and port from RPC URL
    const url = new URL(ETHEREUM_RPC_URL);
    const host = url.hostname;
    const port = url.port || '8545';

    return new Promise((resolve, reject) => {
      // Start anvil with 5s block time and state persistence
      this.anvilProcess = spawn('anvil', [
        '--block-time', '5',
        '--host', host,
        '--port', port,
        '--state-interval', '5',
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

    // Load private key from environment variable
    const privateKey = process.env.PRIVATE_KEY;
    if (!privateKey) {
      throw new Error('PRIVATE_KEY environment variable is required. Please set it in your .env file.');
    }

    // Retry connection to RPC with exponential backoff
    const maxRetries = 5;
    let lastError: any;

    for (let attempt = 0; attempt < maxRetries; attempt++) {
      try {
        this.provider = new ethers.JsonRpcProvider(ETHEREUM_RPC_URL);
        this.wallet = new ethers.Wallet(privateKey, this.provider);

        // Test the connection by getting the network
        await this.provider.getNetwork();

        console.log('Wallet address:', this.wallet.address);
        console.log('Connected to network:', ETHEREUM_RPC_URL);
        return;
      } catch (error: any) {
        lastError = error;

        if (attempt < maxRetries - 1) {
          const waitTime = Math.pow(2, attempt) * 1000; // Exponential backoff: 1s, 2s, 4s, 8s, 16s
          console.warn(`Failed to connect to RPC on attempt ${attempt + 1}/${maxRetries}, retrying in ${waitTime}ms...`);
          console.warn(`Error: ${error.message}`);
          await new Promise(resolve => setTimeout(resolve, waitTime));
        }
      }
    }

    throw new Error(`Failed to connect to RPC after ${maxRetries} attempts: ${lastError.message}`);
  }

  /**
   * Deploy the AgentLog contract
   */
  async deployContract(): Promise<string> {
    if (!this.wallet) {
      throw new Error('Wallet not initialized');
    }

    console.log('Deploying AgentLog contract...');

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
    this.contract = contract as ethers.Contract;

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
  async addLog(data: string): Promise<{ id: bigint; timestamp: bigint; txHash: string }> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    // Retry logic for transient network failures
    const maxRetries = 3;
    let lastError: any;

    for (let attempt = 0; attempt < maxRetries; attempt++) {
      try {
        const tx = await this.contract.addLog(data);
        const receipt = await tx.wait();

        // Parse the LogCreated event
        const event = receipt.logs
          .map((log: any) => {
            try {
              return this.contract!.interface.parseLog(log);
            } catch (e) {
              console.error('Failed to parse log:', e);
              return null;
            }
          })
          .find((e: any) => e && e.name === 'LogCreated');

        if (!event) {
          console.error('Available events:', receipt.logs.map((log: any) => {
            try {
              return this.contract!.interface.parseLog(log);
            } catch {
              return 'unparseable';
            }
          }));
          throw new Error('LogCreated event not found');
        }

        return {
          id: event.args.id,
          timestamp: event.args.timestamp,
          txHash: receipt.hash
        };
      } catch (error: any) {
        lastError = error;

        // Check if it's a network error that we should retry
        if (error.code === 'ECONNREFUSED' ||
            error.code === 'NETWORK_ERROR' ||
            error.code === 'ETIMEDOUT' ||
            (error.message && error.message.includes('could not detect network'))) {

          if (attempt < maxRetries - 1) {
            const waitTime = Math.pow(2, attempt) * 1000; // Exponential backoff
            console.warn(`Network error on attempt ${attempt + 1}/${maxRetries}, retrying in ${waitTime}ms...`, error.message);
            await new Promise(resolve => setTimeout(resolve, waitTime));
            continue;
          }
        }

        // If not a retryable error, or we've exhausted retries, throw
        throw error;
      }
    }

    throw lastError;
  }

  /**
   * Get a log entry by ID for a specific agent
   */
  async getLog(agent: string, id: number): Promise<any> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    return await this.contract.getLog(agent, id);
  }

  /**
   * Get total log count for a specific agent
   */
  async getLogCount(agent: string): Promise<bigint> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    return await this.contract.logCount(agent);
  }

  /**
   * Get logs in a range for a specific agent
   */
  async getLogRange(agent: string, start: number, end: number): Promise<any[]> {
    if (!this.contract) {
      throw new Error('Contract not initialized');
    }

    return await this.contract.getLogRange(agent, start, end);
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
   * Helper to save contract address to .env file
   */
  private saveContractAddress(address: string): void {
    const envPath = path.join(process.cwd(), '.env');
    try {
      let envContent = '';

      // Read existing .env file if it exists
      if (fs.existsSync(envPath)) {
        envContent = fs.readFileSync(envPath, 'utf-8');
      }

      // Check if CONTRACT_ADDRESS already exists in the file
      if (envContent.includes('CONTRACT_ADDRESS=')) {
        // Update existing CONTRACT_ADDRESS
        envContent = envContent.replace(
          /CONTRACT_ADDRESS=.*/,
          `CONTRACT_ADDRESS=${address}`
        );
      } else {
        // Append CONTRACT_ADDRESS to the end
        envContent += `\n# Contract address (automatically set after deployment)\nCONTRACT_ADDRESS=${address}\n`;
      }

      fs.writeFileSync(envPath, envContent);
      console.log('✓ Contract address saved to .env file');
    } catch (error) {
      console.warn('Warning: Could not save contract address to .env file:', error);
    }
  }

  /**
   * Helper to load contract address from .env file
   */
  static loadContractAddress(): string | null {
    return process.env.CONTRACT_ADDRESS || null;
  }

  /**
   * Get contract bytecode from compiled artifacts
   */
  private getContractBytecode(): string {
    try {
      const artifactPath = path.join(process.cwd(), 'out', 'AgentLog.sol', 'AgentLog.json');
      const artifact = JSON.parse(fs.readFileSync(artifactPath, 'utf-8'));

      if (!artifact.bytecode || !artifact.bytecode.object) {
        throw new Error('Bytecode not found in artifact');
      }

      return artifact.bytecode.object;
    } catch (error) {
      console.error('Failed to load contract bytecode from compiled artifacts:', error);
      throw new Error('Contract artifacts not found. Please run: forge build');
    }
  }
}
