/**
 * SafeClaw TypeScript SDK
 *
 * A client library for interacting with the SafeClaw AgentTrace Sidecar.
 * Provides observability and provable execution for AI agents.
 */

export interface LogResult {
  success: boolean;
  id: string;
  hash: string;
  txHash: string;
}

export interface LogEntry {
  id: string;
  timestamp: string;
  timestampISO: string;
  agent: string;
  action: string;
  metadata: string;
  previousHash: string;
  currentHash: string;
}

export interface EntriesResponse {
  entries: LogEntry[];
  total: number;
  start: number;
  limit: number;
  returned: number;
}

export interface VerificationResult {
  valid: boolean;
  entryCount: number;
  lastHash: string | null;
  errors: string[];
  verifiedUpTo: number;
}

export interface EntryVerificationResult {
  id: number;
  valid: boolean;
  error?: string;
}

export interface ProofOfExecution {
  entryId: string;
  timestamp: string;
  timestampISO: string;
  action: string;
  metadata: string;
  currentHash: string;
  previousHash: string;
  chainValid: boolean;
  blockNumber: string | null;
  transactionHash: string | null;
}

export interface HealthResult {
  healthy: boolean;
  lastLogTime: string | null;
  timeSinceLastLog: number | null;
  gaps: Array<{ fromId: number; toId: number; gapSeconds: number }>;
  expectedInterval: number;
  tolerance: number;
}

export interface TamperingResult {
  tampered: boolean;
  issues: string[];
  details: any;
}

export interface Summary {
  totalEntries: number;
  chainValid: boolean;
  lastHash: string | null;
  lastEntry: {
    id: string;
    timestamp: string;
    timestampISO: string;
    agent: string;
    action: string;
  } | null;
  contractAddress: string | null;
}

export interface StatusResult {
  status: string;
  contractAddress: string | null;
  totalEntries: string;
  anvilRPC: string;
}

export class SafeClawClient {
  private baseUrl: string;

  /**
   * Initialize the SafeClaw client
   * @param baseUrl The base URL of the SafeClaw sidecar (default: http://localhost:3000)
   */
  constructor(baseUrl: string = "http://localhost:3000") {
    this.baseUrl = baseUrl.replace(/\/$/, "");
  }

  /**
   * Submit a new log entry
   * @param action The action being logged
   * @param metadata Additional metadata (optional)
   * @returns The log result
   */
  async log(action: string, metadata?: any): Promise<LogResult> {
    const response = await fetch(`${this.baseUrl}/log`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ action, metadata: metadata || {} }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || "Failed to log action");
    }

    return response.json();
  }

  /**
   * Get log entries with pagination
   * @param start Start index (1-based, default: 1)
   * @param limit Maximum number of entries to return (default: 100)
   * @returns The entries response
   */
  async getEntries(start: number = 1, limit: number = 100): Promise<EntriesResponse> {
    const response = await fetch(
      `${this.baseUrl}/entries?start=${start}&limit=${limit}`
    );

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || "Failed to get entries");
    }

    return response.json();
  }

  /**
   * Get a specific log entry by ID
   * @param entryId The entry ID
   * @returns The log entry
   */
  async getEntry(entryId: number): Promise<LogEntry> {
    const response = await fetch(`${this.baseUrl}/entry/${entryId}`);

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || "Failed to get entry");
    }

    return response.json();
  }

  /**
   * Verify the integrity of the entire chain
   * @param upToId Verify up to this entry ID (optional, default: all)
   * @returns The verification result
   */
  async verifyChain(upToId?: number): Promise<VerificationResult> {
    const url = upToId
      ? `${this.baseUrl}/verify?upToId=${upToId}`
      : `${this.baseUrl}/verify`;

    const response = await fetch(url);

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || "Failed to verify chain");
    }

    return response.json();
  }

  /**
   * Verify a specific log entry
   * @param entryId The entry ID to verify
   * @returns The verification result
   */
  async verifyEntry(entryId: number): Promise<EntryVerificationResult> {
    const response = await fetch(`${this.baseUrl}/verify/${entryId}`);

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || "Failed to verify entry");
    }

    return response.json();
  }

  /**
   * Get a proof of execution for a specific log entry
   * @param entryId The entry ID
   * @returns The proof of execution
   */
  async getProof(entryId: number): Promise<ProofOfExecution> {
    const response = await fetch(`${this.baseUrl}/proof/${entryId}`);

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || "Failed to get proof");
    }

    return response.json();
  }

  /**
   * Check the health of the agent (heartbeat verification)
   * @param expectedInterval Expected interval between logs in seconds (default: 60)
   * @param tolerance Tolerance in seconds (default: 30)
   * @returns The health result
   */
  async checkHealth(
    expectedInterval: number = 60,
    tolerance: number = 30
  ): Promise<HealthResult> {
    const response = await fetch(
      `${this.baseUrl}/health?expectedInterval=${expectedInterval}&tolerance=${tolerance}`
    );

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || "Failed to check health");
    }

    return response.json();
  }

  /**
   * Detect potential tampering
   * @returns The tampering detection result
   */
  async detectTampering(): Promise<TamperingResult> {
    const response = await fetch(`${this.baseUrl}/tampering`);

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || "Failed to detect tampering");
    }

    return response.json();
  }

  /**
   * Get a summary of the logs and chain status
   * @returns The summary
   */
  async getSummary(): Promise<Summary> {
    const response = await fetch(`${this.baseUrl}/summary`);

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || "Failed to get summary");
    }

    return response.json();
  }

  /**
   * Get the status of the sidecar service
   * @returns The status
   */
  async getStatus(): Promise<StatusResult> {
    const response = await fetch(`${this.baseUrl}/status`);

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || "Failed to get status");
    }

    return response.json();
  }
}

/**
 * Convenience function to log an action without creating a client instance
 * @param action The action being logged
 * @param metadata Additional metadata (optional)
 * @param baseUrl The base URL of the SafeClaw sidecar
 * @returns The log result
 */
export async function logAction(
  action: string,
  metadata?: any,
  baseUrl: string = "http://localhost:3000"
): Promise<LogResult> {
  const client = new SafeClawClient(baseUrl);
  return client.log(action, metadata);
}

export default SafeClawClient;
