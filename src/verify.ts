import { ChainManager } from './chain';
import { ethers } from 'ethers';

export interface LogEntry {
  id: bigint;
  timestamp: bigint;
  agent: string;
  action: string;
  metadata: string;
  previousHash: string;
  currentHash: string;
}

export interface VerificationResult {
  valid: boolean;
  entryCount: number;
  lastHash: string | null;
  errors: string[];
}

export interface ProofOfExecution {
  entryId: bigint;
  timestamp: bigint;
  action: string;
  metadata: string;
  currentHash: string;
  previousHash: string;
  chainValid: boolean;
  blockNumber: bigint | null;
  transactionHash: string | null;
}

export class VerificationService {
  constructor(private chainManager: ChainManager) {}

  /**
   * Verify a single log entry
   */
  async verifyEntry(id: number): Promise<{ valid: boolean; error?: string }> {
    try {
      const valid = await this.chainManager.verifyLog(id);
      return { valid };
    } catch (error: any) {
      return { valid: false, error: error.message };
    }
  }

  /**
   * Verify the entire chain integrity
   */
  async verifyChainIntegrity(upToId?: number): Promise<VerificationResult> {
    const errors: string[] = [];
    let entryCount = 0;
    let lastHash: string | null = null;

    try {
      // Get total entry count
      const count = await this.chainManager.getLogCount();
      entryCount = Number(count);

      if (entryCount === 0) {
        return {
          valid: true,
          entryCount: 0,
          lastHash: null,
          errors: []
        };
      }

      // Verify the chain
      const targetId = upToId && upToId <= entryCount ? upToId : entryCount;
      const valid = await this.chainManager.verifyChain(targetId);

      if (!valid) {
        errors.push('Chain verification failed: hash chain is broken');
      }

      // Get last entry hash
      if (entryCount > 0) {
        const lastEntry = await this.chainManager.getLog(targetId);
        lastHash = lastEntry.currentHash;
      }

      return {
        valid,
        entryCount,
        lastHash,
        errors
      };
    } catch (error: any) {
      errors.push(`Verification error: ${error.message}`);
      return {
        valid: false,
        entryCount,
        lastHash,
        errors
      };
    }
  }

  /**
   * Generate a proof of execution for a specific log entry
   */
  async generateProof(entryId: number): Promise<ProofOfExecution> {
    const entry = await this.chainManager.getLog(entryId);
    const chainValid = await this.chainManager.verifyChain(entryId);

    // TODO: Get actual block number and transaction hash from chain
    // This would require storing additional metadata or querying the provider

    return {
      entryId: entry.id,
      timestamp: entry.timestamp,
      action: entry.action,
      metadata: entry.metadata,
      currentHash: entry.currentHash,
      previousHash: entry.previousHash,
      chainValid,
      blockNumber: null,
      transactionHash: null
    };
  }

  /**
   * Verify that logs have been consistently posted (heartbeat check)
   */
  async checkHeartbeat(
    expectedIntervalSeconds: number,
    toleranceSeconds: number = 30
  ): Promise<{
    healthy: boolean;
    lastLogTime: Date | null;
    timeSinceLastLog: number | null;
    gaps: Array<{ fromId: number; toId: number; gapSeconds: number }>;
  }> {
    const count = await this.chainManager.getLogCount();
    const numEntries = Number(count);

    if (numEntries === 0) {
      return {
        healthy: false,
        lastLogTime: null,
        timeSinceLastLog: null,
        gaps: []
      };
    }

    // Get all entries
    const entries = await this.chainManager.getLogRange(1, numEntries);
    const gaps: Array<{ fromId: number; toId: number; gapSeconds: number }> = [];

    // Check for gaps between entries
    for (let i = 1; i < entries.length; i++) {
      const prevEntry = entries[i - 1];
      const currEntry = entries[i];

      const timeDiff = Number(currEntry.timestamp) - Number(prevEntry.timestamp);

      if (timeDiff > expectedIntervalSeconds + toleranceSeconds) {
        gaps.push({
          fromId: Number(prevEntry.id),
          toId: Number(currEntry.id),
          gapSeconds: timeDiff
        });
      }
    }

    // Get last log time
    const lastEntry = entries[entries.length - 1];
    const lastLogTime = new Date(Number(lastEntry.timestamp) * 1000);
    const now = Date.now();
    const timeSinceLastLog = Math.floor((now - lastLogTime.getTime()) / 1000);

    // Agent is healthy if no recent gaps and last log is recent
    const healthy = gaps.length === 0 && timeSinceLastLog <= expectedIntervalSeconds + toleranceSeconds;

    return {
      healthy,
      lastLogTime,
      timeSinceLastLog,
      gaps
    };
  }

  /**
   * Detect potential tampering by checking for:
   * - Broken hash chains
   * - Missing entries
   * - Suspicious gaps in logging
   */
  async detectTampering(): Promise<{
    tampered: boolean;
    issues: string[];
    details: any;
  }> {
    const issues: string[] = [];
    const details: any = {};

    // Check chain integrity
    const chainResult = await this.verifyChainIntegrity();
    if (!chainResult.valid) {
      issues.push('Chain integrity check failed');
      details.chainErrors = chainResult.errors;
    }

    // Check for heartbeat anomalies (assuming 60s expected interval)
    const heartbeat = await this.checkHeartbeat(60, 30);
    if (!heartbeat.healthy) {
      issues.push('Heartbeat anomalies detected');
      details.heartbeat = heartbeat;
    }

    // Check for suspicious gaps
    if (heartbeat.gaps.length > 0) {
      issues.push(`Found ${heartbeat.gaps.length} suspicious logging gaps`);
    }

    return {
      tampered: issues.length > 0,
      issues,
      details
    };
  }

  /**
   * Get a summary of all logs with verification status
   */
  async getSummary(): Promise<{
    totalEntries: number;
    chainValid: boolean;
    lastEntry: LogEntry | null;
    lastHash: string | null;
  }> {
    const count = await this.chainManager.getLogCount();
    const totalEntries = Number(count);

    if (totalEntries === 0) {
      return {
        totalEntries: 0,
        chainValid: true,
        lastEntry: null,
        lastHash: null
      };
    }

    const chainValid = await this.chainManager.verifyChain(0);
    const lastEntry = await this.chainManager.getLog(totalEntries);

    return {
      totalEntries,
      chainValid,
      lastEntry,
      lastHash: lastEntry.currentHash
    };
  }

  /**
   * Compute hash for an entry (for external verification)
   */
  static computeEntryHash(entry: {
    id: bigint;
    timestamp: bigint;
    agent: string;
    action: string;
    metadata: string;
    previousHash: string;
  }): string {
    const encoded = ethers.solidityPacked(
      ['uint256', 'uint256', 'address', 'string', 'string', 'bytes32'],
      [entry.id, entry.timestamp, entry.agent, entry.action, entry.metadata, entry.previousHash]
    );
    return ethers.keccak256(encoded);
  }
}
