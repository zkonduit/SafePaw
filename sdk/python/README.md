# SafeClaw Python SDK

Python client library for interacting with SafeClaw AgentTrace Sidecar.

## Installation

```bash
pip install safeclaw
```

Or install from source:

```bash
cd sdk/python
pip install -e .
```

## Usage

### Basic Usage

```python
from safeclaw import SafeClawClient

# Initialize client
client = SafeClawClient(base_url="http://localhost:3000")

# Log an action
result = client.log("agent_started", {"version": "1.0", "model": "gpt-4"})
print(f"Logged with ID: {result['id']}")

# Get entries
entries = client.get_entries(start=1, limit=10)
print(f"Total entries: {entries['total']}")

# Verify chain integrity
verification = client.verify_chain()
print(f"Chain valid: {verification['valid']}")

# Get a specific entry
entry = client.get_entry(1)
print(f"Entry: {entry}")

# Get proof of execution
proof = client.get_proof(1)
print(f"Proof: {proof}")

# Check health
health = client.check_health(expected_interval=60, tolerance=30)
print(f"Agent healthy: {health['healthy']}")

# Detect tampering
tampering = client.detect_tampering()
print(f"Tampering detected: {tampering['tampered']}")
```

### Convenience Function

```python
from safeclaw import log_action

# Quick logging without creating a client
log_action("task_completed", {"task_id": "123", "status": "success"})
```

## API Reference

### SafeClawClient

#### `__init__(base_url: str = "http://localhost:3000")`

Initialize the SafeClaw client.

#### `log(action: str, metadata: Optional[Dict] = None) -> Dict`

Submit a new log entry.

#### `get_entries(start: int = 1, limit: int = 100) -> Dict`

Get log entries with pagination.

#### `get_entry(entry_id: int) -> Dict`

Get a specific log entry by ID.

#### `verify_chain(up_to_id: Optional[int] = None) -> Dict`

Verify the integrity of the entire chain.

#### `verify_entry(entry_id: int) -> Dict`

Verify a specific log entry.

#### `get_proof(entry_id: int) -> Dict`

Get a proof of execution for a specific log entry.

#### `check_health(expected_interval: int = 60, tolerance: int = 30) -> Dict`

Check the health of the agent (heartbeat verification).

#### `detect_tampering() -> Dict`

Detect potential tampering.

#### `get_summary() -> Dict`

Get a summary of the logs and chain status.

#### `get_status() -> Dict`

Get the status of the sidecar service.

## License

MIT
