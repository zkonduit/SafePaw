# SafeClaw TypeScript SDK

TypeScript client library for interacting with SafeClaw AgentTrace Sidecar.

## Installation

```bash
npm install @safeclaw/sdk
```

Or with yarn:

```bash
yarn add @safeclaw/sdk
```

## Usage

### Basic Usage

```typescript
import { SafeClawClient } from '@safeclaw/sdk';

// Initialize client
const client = new SafeClawClient('http://localhost:3000');

// Log an action
const result = await client.log('agent_started', { version: '1.0', model: 'gpt-4' });
console.log(`Logged with ID: ${result.id}`);

// Get entries
const entries = await client.getEntries(1, 10);
console.log(`Total entries: ${entries.total}`);

// Verify chain integrity
const verification = await client.verifyChain();
console.log(`Chain valid: ${verification.valid}`);

// Get a specific entry
const entry = await client.getEntry(1);
console.log('Entry:', entry);

// Get proof of execution
const proof = await client.getProof(1);
console.log('Proof:', proof);

// Check health
const health = await client.checkHealth(60, 30);
console.log(`Agent healthy: ${health.healthy}`);

// Detect tampering
const tampering = await client.detectTampering();
console.log(`Tampering detected: ${tampering.tampered}`);
```

### Convenience Function

```typescript
import { logAction } from '@safeclaw/sdk';

// Quick logging without creating a client
await logAction('task_completed', { task_id: '123', status: 'success' });
```

## API Reference

### SafeClawClient

#### `constructor(baseUrl?: string)`

Initialize the SafeClaw client.

- `baseUrl` (optional): The base URL of the SafeClaw sidecar (default: `http://localhost:3000`)

#### `log(action: string, metadata?: any): Promise<LogResult>`

Submit a new log entry.

#### `getEntries(start?: number, limit?: number): Promise<EntriesResponse>`

Get log entries with pagination.

#### `getEntry(entryId: number): Promise<LogEntry>`

Get a specific log entry by ID.

#### `verifyChain(upToId?: number): Promise<VerificationResult>`

Verify the integrity of the entire chain.

#### `verifyEntry(entryId: number): Promise<EntryVerificationResult>`

Verify a specific log entry.

#### `getProof(entryId: number): Promise<ProofOfExecution>`

Get a proof of execution for a specific log entry.

#### `checkHealth(expectedInterval?: number, tolerance?: number): Promise<HealthResult>`

Check the health of the agent (heartbeat verification).

#### `detectTampering(): Promise<TamperingResult>`

Detect potential tampering.

#### `getSummary(): Promise<Summary>`

Get a summary of the logs and chain status.

#### `getStatus(): Promise<StatusResult>`

Get the status of the sidecar service.

## TypeScript Types

All response types are fully typed. See the source code for complete type definitions.

## License

MIT
