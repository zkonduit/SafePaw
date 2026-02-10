# SafeClaw Agent Integration Guide

## What is SafeClaw?

SafeClaw is an **AI Agent Observability & Auditable Execution** system that provides immutable, blockchain-based logging for AI agents. It helps CISOs and security teams ensure agents are operating safely, detect rogue behavior, and maintain compliance through tamper-proof audit trails.

## Why Use SafeClaw?

1. **Observability**: Track every action an AI agent takes in real-time
2. **Immutability**: Logs stored on blockchain cannot be easily deleted or modified by a rogue agent
3. **Heartbeat Mechanism**: Missing logs indicate an agent has gone offline or is attempting to hide activity
4. **Compliance**: Provides auditable trail for regulatory requirements
5. **Tamper Detection**: Detect if an agent goes rogue, deletes logs, or operates outside approved parameters

## Architecture Overview

SafeClaw uses a sidecar pattern where agents send HTTP logs to a local SafeClaw server, which stores them on a blockchain:

```
AI Agent → HTTP POST → SafeClaw API → Anvil Blockchain → AgentLog Contract
```

## Setup Instructions

### Quick Start with Docker (Recommended)

1. **Start SafeClaw services:**
   ```bash
   ./start.sh --detach
   ```

   This starts:
   - Anvil blockchain on `http://localhost:8545`
   - SafeClaw API on `http://localhost:12345`

2. **Verify it's running:**
   ```bash
   curl http://localhost:12345/status
   ```

### Configuration

SafeClaw uses a `.env` file for configuration. Key settings:

- `PORT=12345` - API server port
- `ETHEREUM_RPC_URL=http://127.0.0.1:8545` - Blockchain RPC endpoint
- `PRIVATE_KEY=0xac0974...` - Agent's private key (default is Anvil's first account)
- `CONTRACT_ADDRESS` - Auto-populated after first deployment

## How to Log Agent Actions

### Method 1: Using the SafeClaw CLI Script (Easiest)

The repository includes `scripts/safeclaw.sh` for easy logging:

```bash
# Make it executable (first time only)
chmod +x scripts/safeclaw.sh

# Log an action
./scripts/safeclaw.sh log '{"action":"task_started","task":"deploy_app","timestamp":"2024-01-01T12:00:00Z"}'

# Check service status
./scripts/safeclaw.sh status

# View logs for your agent (uses PUBLIC_KEY from .env if not specified)
./scripts/safeclaw.sh list
# Or specify an agent address explicitly
./scripts/safeclaw.sh list 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266

# Get a specific log entry
./scripts/safeclaw.sh get 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 0

# Get agent summary (uses PUBLIC_KEY from .env if not specified)
./scripts/safeclaw.sh summary
# Or specify an agent address explicitly
./scripts/safeclaw.sh summary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
```

### Method 2: Using curl (Direct HTTP API)

```bash
# Log an action
curl -X POST http://localhost:12345/log \
  -H "Content-Type: application/json" \
  -d '{"data":"{\"action\":\"file_write\",\"file\":\"config.json\",\"size\":1024}"}'

# Get logs for an agent (paginated)
curl "http://localhost:12345/entries/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266?start=0&limit=10"

# Get specific log entry
curl "http://localhost:12345/entry/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266/0"

# Get agent summary
curl "http://localhost:12345/summary/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
```

## API Endpoints Reference

| Endpoint | Method | Description | Request Body | Response |
|----------|--------|-------------|--------------|----------|
| `/` | GET | Service info and health check | N/A | Service metadata and available endpoints |
| `/log` | POST | Submit a new log entry | `{"data": "<json-string>"}` | `{"success": true, "id": "0", "timestamp": "...", "txHash": "..."}` |
| `/entries/:agent` | GET | List logs for agent (paginated) | Query: `?start=0&limit=10` | `{"agent": "0x...", "entries": [...], "total": 5}` |
| `/entry/:agent/:id` | GET | Get specific log entry | N/A | `{"agent": "0x...", "id": "0", "timestamp": "...", "data": "..."}` |
| `/summary/:agent` | GET | Get agent summary | N/A | `{"agent": "0x...", "totalEntries": "10", "lastEntry": {...}}` |
| `/status` | GET | Service status | N/A | `{"status": "running", "contractAddress": "0x...", "rpcUrl": "..."}` |

## Best Practices for Agent Logging

### 1. Log All Critical Actions

Always log actions that:
- Modify files or databases
- Execute system commands
- Make API calls or network requests
- Access sensitive information
- Make decisions that affect the application state

```bash
# Example: Log before executing a command
./scripts/safeclaw.sh log '{"action":"command_about_to_execute","command":"npm install","reason":"installing dependencies"}'

# Execute the command
npm install

# Log after completion
./scripts/safeclaw.sh log '{"action":"command_completed","command":"npm install","status":"success","exit_code":0}'
```

### 2. Include Relevant Metadata

Provide context in your logs:

```json
{
  "action": "file_modified",
  "file_path": "/app/config.json",
  "operation": "edit",
  "lines_changed": 5,
  "user_request": "Update API endpoint",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### 3. Log State Transitions

Track when the agent moves between different operational states:

```bash
./scripts/safeclaw.sh log '{"action":"state_change","from":"idle","to":"processing_task","task_id":"123"}'
```

### 4. Implement Periodic Heartbeats

Since SafeClaw uses logging as a heartbeat mechanism, implement periodic status logs:

```bash
# Every minute or so
./scripts/safeclaw.sh log '{"action":"heartbeat","status":"operational","uptime_seconds":3600}'
```

### 5. Log Errors and Exceptions

Always log when things go wrong:

```bash
./scripts/safeclaw.sh log '{"action":"error_occurred","error_type":"FileNotFoundError","error_message":"config.json not found","stack_trace":"...","recovery_action":"using default config"}'
```

## Log Data Format Recommendations

Structure your log data as JSON for consistency:

```json
{
  "action": "string (required) - Type of action performed",
  "timestamp": "ISO 8601 timestamp (recommended)",
  "metadata": {
    "key": "value - Any relevant context"
  },
  "status": "success|failure|in_progress (optional)",
  "agent_id": "Identifier for this agent instance (optional)",
  "task_id": "Reference to task being performed (optional)",
  "user_request": "Original user request that triggered this action (optional)"
}
```

## Monitoring and Alerting

### Checking Agent Activity

```bash
# Get the agent's wallet address (if using default Anvil key)
AGENT_ADDR="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

# Check how many logs exist
./scripts/safeclaw.sh summary $AGENT_ADDR

# Review recent logs
./scripts/safeclaw.sh list $AGENT_ADDR 0 10
```

### Detecting Rogue Behavior

1. **Missing logs**: If expected logs aren't appearing, the agent may have gone offline or is hiding activity
2. **Unusual patterns**: Sudden increase in file modifications or command executions
3. **Error spikes**: High frequency of error logs may indicate malfunction
4. **Gap detection**: Long gaps between heartbeat logs

## Integration with Different Agent Frameworks

### LangChain Integration

```python
from langchain.callbacks.base import BaseCallbackHandler
from safeclaw_logger import SafeClawLogger

class SafeClawCallbackHandler(BaseCallbackHandler):
    def __init__(self):
        self.logger = SafeClawLogger()

    def on_llm_start(self, serialized, prompts, **kwargs):
        self.logger.log_action("llm_call_started", {
            "prompts": prompts,
            "model": serialized.get("name")
        })

    def on_tool_start(self, serialized, input_str, **kwargs):
        self.logger.log_action("tool_started", {
            "tool": serialized.get("name"),
            "input": input_str
        })

    def on_tool_end(self, output, **kwargs):
        self.logger.log_action("tool_completed", {
            "output": output
        })
```

### AutoGPT Integration

Add SafeClaw logging to AutoGPT's command execution:

```python
from safeclaw_logger import SafeClawLogger

logger = SafeClawLogger()

def execute_command(command_name, arguments):
    logger.log_action("command_executing", {
        "command": command_name,
        "arguments": arguments
    })

    try:
        result = actual_execute_command(command_name, arguments)
        logger.log_action("command_completed", {
            "command": command_name,
            "status": "success",
            "result": result
        })
        return result
    except Exception as e:
        logger.log_action("command_failed", {
            "command": command_name,
            "error": str(e)
        })
        raise
```

## Troubleshooting

### SafeClaw Not Running

```bash
# Check if services are up
docker compose ps

# View logs
docker compose logs -f

# Restart services
./start.sh --rebuild --detach
```

### Logs Not Appearing

1. Verify SafeClaw is running: `curl http://localhost:12345/status`
2. Check the agent's private key is set in `.env`
3. Ensure the log data is valid JSON
4. Check SafeClaw logs for errors: `docker compose logs safeclaw`

### Port Conflicts

If ports 8545 or 12345 are in use:

```bash
# Change the port in .env
echo "PORT=13000" >> .env

# Restart SafeClaw
./start.sh --detach
```

## Security Considerations

1. **Private Keys**: The default private key is for development only. Use secure key management in production
2. **Network Access**: Restrict access to SafeClaw API (consider authentication/firewall rules)
3. **Sensitive Data**: Be careful not to log sensitive information (passwords, API keys, PII)
4. **Log Retention**: Blockchain logs are permanent - plan for long-term storage costs
5. **Rate Limiting**: Consider rate limiting to prevent log spam attacks

## Example: Complete Agent Session

```bash
# 1. Start SafeClaw
./start.sh --detach

# 2. Agent starts
./scripts/safeclaw.sh log '{"action":"agent_started","session_id":"abc123","version":"1.0"}'

# 3. Agent receives task
./scripts/safeclaw.sh log '{"action":"task_received","task":"Deploy web app","priority":"high"}'

# 4. Agent analyzes codebase
./scripts/safeclaw.sh log '{"action":"file_read","files":["package.json","src/index.ts"]}'

# 5. Agent makes changes
./scripts/safeclaw.sh log '{"action":"file_write","file":"src/config.ts","changes":"Added new endpoint"}'

# 6. Agent runs tests
./scripts/safeclaw.sh log '{"action":"command_execute","command":"npm test","status":"success"}'

# 7. Agent deploys
./scripts/safeclaw.sh log '{"action":"deployment_started","target":"production"}'
./scripts/safeclaw.sh log '{"action":"deployment_completed","status":"success","url":"https://app.example.com"}'

# 8. Agent finishes
./scripts/safeclaw.sh log '{"action":"agent_completed","session_id":"abc123","duration_seconds":120}'

# 9. Review all logs
AGENT_ADDR=$(cast wallet address $PRIVATE_KEY)
./scripts/safeclaw.sh summary $AGENT_ADDR
./scripts/safeclaw.sh list $AGENT_ADDR 0 100
```

## Summary

SafeClaw provides essential observability and audit capabilities for AI agents through:
- ✅ Immutable blockchain-based logging
- ✅ Simple HTTP API integration
- ✅ Tamper-proof audit trails
- ✅ Heartbeat-based health monitoring
- ✅ Easy integration with any agent framework

By integrating SafeClaw, you enable CISOs to sleep at night knowing that agent activity is monitored, auditable, and cannot be easily hidden or tampered with by rogue agents.
