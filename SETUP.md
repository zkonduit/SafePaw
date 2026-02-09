# SafeClaw Setup Guide

This guide will walk you through setting up the SafeClaw AgentTrace Sidecar for AI agent observability and provable execution.

## Prerequisites

Before you begin, ensure you have the following installed:

### Required
- **Node.js** (v18 or higher) - [Install Node.js](https://nodejs.org/)
- **npm** (comes with Node.js)
- **Foundry** (for Solidity compilation and Anvil) - [Install Foundry](https://book.getfoundry.sh/getting-started/installation)

### Optional
- **Docker** and **Docker Compose** - For containerized deployment
- **jq** - For CLI usage (macOS: `brew install jq`, Linux: `apt-get install jq`)

## Installation Methods

### Method 1: Local Development Setup

1. **Clone the repository** (if not already done):
   ```bash
   cd SafeClaw
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Install Foundry** (if not already installed):
   ```bash
   curl -L https://foundry.paradigm.xyz | bash
   foundryup
   ```

4. **Compile Solidity contracts**:
   ```bash
   forge build
   ```

5. **Build TypeScript**:
   ```bash
   npm run build
   ```

6. **Start the sidecar**:
   ```bash
   npm run dev
   ```

   The sidecar will:
   - Start Anvil on `http://localhost:8545`
   - Deploy the AgentLog contract
   - Start the HTTP API on `http://localhost:3000`

### Method 2: Docker Setup

1. **Build the Docker image**:
   ```bash
   docker-compose build
   ```

2. **Start the services**:
   ```bash
   docker-compose up -d
   ```

3. **Check the status**:
   ```bash
   docker-compose ps
   ```

4. **View logs**:
   ```bash
   docker-compose logs -f safeclaw
   ```

5. **Stop the services**:
   ```bash
   docker-compose down
   ```

## Verification

Once the sidecar is running, verify it's working:

```bash
# Check service status
curl http://localhost:3000/status

# Get summary
curl http://localhost:3000/summary
```

You should see a JSON response indicating the service is running.

## Using the SDKs

### Python SDK

1. **Install the SDK**:
   ```bash
   cd sdk/python
   pip install -e .
   ```

2. **Use in your code**:
   ```python
   from safeclaw import SafeClawClient

   client = SafeClawClient("http://localhost:3000")
   result = client.log("agent_started", {"version": "1.0"})
   print(f"Logged with ID: {result['id']}")
   ```

### TypeScript SDK

1. **Install the SDK**:
   ```bash
   cd sdk/typescript
   npm install
   npm run build
   ```

2. **Use in your code**:
   ```typescript
   import { SafeClawClient } from '@safeclaw/sdk';

   const client = new SafeClawClient('http://localhost:3000');
   const result = await client.log('agent_started', { version: '1.0' });
   console.log(`Logged with ID: ${result.id}`);
   ```

## Using the CLI

1. **Make the CLI script accessible**:
   ```bash
   # Option 1: Add to PATH
   export PATH="$PATH:$(pwd)/cli"

   # Option 2: Create a symlink
   sudo ln -s $(pwd)/cli/safeclaw.sh /usr/local/bin/safeclaw
   ```

2. **Use the CLI**:
   ```bash
   # Log an action
   safeclaw log "agent_started" '{"version":"1.0"}'

   # List entries
   safeclaw entries

   # Verify chain
   safeclaw verify

   # Get summary
   safeclaw summary

   # Check help
   safeclaw help
   ```

## API Endpoints

Once running, the following endpoints are available:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | Service info and endpoint list |
| `/log` | POST | Submit a new log entry |
| `/entries` | GET | List log entries (paginated) |
| `/entry/:id` | GET | Get a specific log entry |
| `/verify` | GET | Verify chain integrity |
| `/verify/:id` | GET | Verify a specific entry |
| `/proof/:id` | GET | Get proof of execution |
| `/health` | GET | Check agent health (heartbeat) |
| `/tampering` | GET | Detect potential tampering |
| `/summary` | GET | Get summary of logs and status |
| `/status` | GET | Get service status |

## Configuration

### Environment Variables

Create a `.env` file (use `.env.example` as template):

```bash
cp .env.example .env
```

Available configuration:
- `PORT` - HTTP API port (default: 3000)
- `HOST` - HTTP API host (default: 0.0.0.0)
- `NODE_ENV` - Environment (development/production)

## Integrating with Your Agent

### Example: Simple Integration

```python
# At the start of your agent
from safeclaw import log_action

log_action("agent_started", {"version": "1.0", "model": "gpt-4"})

# Log important actions
log_action("task_started", {"task_id": "123", "type": "research"})
log_action("tool_called", {"tool": "web_search", "query": "latest news"})
log_action("task_completed", {"task_id": "123", "status": "success"})

# At shutdown
log_action("agent_stopped", {"reason": "user_request"})
```

### Example: Heartbeat Monitoring

```python
import time
from safeclaw import SafeClawClient

client = SafeClawClient()

# Send heartbeat every 30 seconds
while True:
    client.log("heartbeat", {"timestamp": time.time()})
    time.sleep(30)
```

## Monitoring and Verification

### Check Chain Integrity

```bash
curl http://localhost:3000/verify
```

### Detect Tampering

```bash
curl http://localhost:3000/tampering
```

### Check Agent Health

```bash
# Expects logs every 60 seconds (±30s tolerance)
curl "http://localhost:3000/health?expectedInterval=60&tolerance=30"
```

## Troubleshooting

### Anvil Not Starting

If Anvil fails to start:
1. Check if port 8545 is already in use: `lsof -i :8545`
2. Ensure Foundry is installed: `anvil --version`
3. Check logs for error messages

### Contract Deployment Fails

If contract deployment fails:
1. Ensure contracts are compiled: `forge build`
2. Check the `out/` directory exists and contains compiled contracts
3. Delete `.safeclaw-config.json` to force redeployment

### Port Already in Use

If port 3000 is in use:
```bash
# Find the process
lsof -i :3000

# Change the port
export PORT=3001
npm run dev
```

## Development

### Running Tests

```bash
npm test
```

### Linting

```bash
npm run lint
```

### Hot Reload Development

```bash
npm run dev
```

## Production Deployment

### Using Docker

For production deployment, use Docker Compose:

```bash
docker-compose -f docker-compose.yml up -d
```

### Using PM2

For process management:

```bash
npm install -g pm2
npm run build
pm2 start dist/index.js --name safeclaw
```

## Security Considerations

1. **Private Keys**: Never expose private keys in production
2. **Network Access**: Restrict access to Anvil RPC (port 8545) in production
3. **API Authentication**: Consider adding authentication to the HTTP API
4. **Log Sensitivity**: Be careful about logging sensitive information

## Next Steps

1. Integrate SafeClaw with your AI agent
2. Set up monitoring dashboards using the API endpoints
3. Configure alerting for tampering detection
4. Review logs regularly for compliance

## Support

For issues, questions, or contributions, please visit the GitHub repository.
