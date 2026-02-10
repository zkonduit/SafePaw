<div align="center">
  <img src="mascot/safeclaw.png" alt="SafeClaw" width="400"/>
</div>

# SafeClaw

AI Agent Observability & Auditable Execution - Because your CISO needs to sleep at night.

## Problems
1. Everyone will be using AI in their workflows. Agents are a CISO's nightmare. CEO wants everyone to be using agents for productivity gains. CISO needs to prevent major screw ups and ensure compliance.
2. Skills.md cannot be trusted, and can be a malware vector even if signed as prompt injection backdoors could be introduced.
3. Needs an observability and tracing solution that cannot be deleted, modified by the AI easily.

## Solutions
1. SafeClaw provides observability and provable execution
2. Logs are stored in a blockchain which is shared across a cluster to prevent easy deletion
3. This helps to detect if an agent goes rogue, deletes logs, or goes offline

## Architecture
1. Note posting of logs is basically a heartbeat mechanism, if the agent isn't logging we can assume that it's offline
```
┌─────────────────────────────────────────────────────────────┐
│                     Any Agent System                         │
│  (OpenClaw, LangChain, AutoGPT, custom, whatever)           │
│                           │                                  │
│                    HTTP POST /log                            │
└───────────────────────────┼─────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   SafeClaw Sidecar                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  HTTP API (:12345) [Docker Container]                │   │
│  │  POST /log        - submit action                    │   │
│  │  GET /entries     - list entries                     │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                  │
│               Connects to host.docker.internal:8545          │
└───────────────────────────┼─────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Host Machine                            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Anvil (:8545)                                       │   │
│  │  - Started by start.sh script                        │   │
│  │  - 5s block time                                     │   │
│  │  - Logs to anvil.log                                 │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  AgentLog Contract (auto-deployed)                   │   │
│  │  - Hash chain of entries                             │   │
│  │  - Verification built-in                             │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘

```

## Project Directory
```
safeclaw/
├── docker-compose.yml
├── Dockerfile
├── package.json
├── src/
│   ├── index.ts          # Main sidecar server
│   ├── chain.ts          # Anvil + contract management
│   ├── api.ts            # HTTP endpoints
│   └── verify.ts         # Verification logic
├── contracts/
│   └── AgentLog.sol      # Solidity contract
├── scripts/
│   └── safeclaw.sh       # CLI wrapper for agents
└── out/
    └── AgentLog.sol/
        └── AgentLog.json # Compiled contract artifacts
```

---

# Setup Guide

## Quick Start

Choose your preferred method to run SafeClaw:

### Option 1: Docker (Recommended)

The easiest way to get started. No need to install Node.js or manage dependencies!

**Prerequisites:**
- **Docker** - [Install Docker](https://docs.docker.com/get-docker/)
- **Docker Compose** - Usually included with Docker Desktop

**Start SafeClaw:**
```bash
# Start all services (Anvil on host + SafeClaw API in Docker)
./start.sh --detach

# Or run in foreground (see logs)
./start.sh
```

The `start.sh` script will:
- Check if Docker and Docker Compose are installed
- Verify contracts are compiled (compiles if needed)
- Start Anvil on the host machine (if not already running)
- Start SafeClaw API in Docker container
- Display service URLs and helpful commands

**Services will be available at:**
- Anvil: `http://localhost:8545` (runs on host machine)
- SafeClaw API: `http://localhost:12345` (runs in Docker)

**Useful commands:**
```bash
# Stop all services (including Anvil)
./stop.sh

# View SafeClaw logs
docker compose logs -f

# View Anvil logs
tail -f anvil.log

# View status
docker compose ps

# Rebuild and restart
./start.sh --rebuild --detach
```

### Option 2: Local Development

For development with hot reload and direct access to source code.

**Prerequisites:**
- **Node.js** (v18 or higher) - [Install Node.js](https://nodejs.org/)
- **Foundry** (for Anvil) - Install with: `curl -L https://foundry.paradigm.xyz | bash && foundryup`

**Setup:**

```bash
# 1. Install
pnpm install

# 2. Compile contract
pnpm compile-contracts

# 3. Start the server and anvil
pnpm start

# Or for development with hot reload:
pnpm run dev
```

The server automatically:
- Deploys the AgentLog contract and updates the .env file if a contract doesn't exist already
- Starts the HTTP API on `http://localhost:12345`

**Note**: To start embedded Anvil, set `START_ANVIL=true` in your `.env` file. Otherwise, make sure you have Anvil or another Ethereum node running at the `ETHEREUM_RPC_URL`.

### Running with External Anvil

If you prefer to manage Anvil separately:

```bash
# Terminal 1: Start Anvil manually
anvil --block-time 1

# Terminal 2: Start SafeClaw (with START_ANVIL=false in .env)
pnpm start
```

This is useful for:
- Development workflows where you want to control Anvil lifecycle
- Running multiple SafeClaw instances against the same chain
- Using a persistent Anvil instance across restarts

## API Endpoints

Once running, the following endpoints are available:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | Service info and endpoint list |
| `/log` | POST | Submit a new log entry |
| `/entries/:agent` | GET | List log entries for a specific agent (paginated) |
| `/entry/:agent/:id` | GET | Get a specific log entry by agent and ID |
| `/summary/:agent` | GET | Get summary of logs for a specific agent |
| `/status` | GET | Get service status |

### API Examples

#### Submit a Log Entry
```bash
curl -X POST http://localhost:12345/log \
  -H "Content-Type: application/json" \
  -d '{"data":"{\"action\":\"task_started\",\"task\":\"deploy_app\"}"}'
```

#### Get Logs for an Agent
```bash
curl http://localhost:12345/entries/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266?start=0&limit=10
```

#### Get Specific Log Entry
```bash
curl http://localhost:12345/entry/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266/0
```

#### Get Agent Summary
```bash
curl http://localhost:12345/summary/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
```

## Using the SafeClaw CLI Script

For easier interaction with SafeClaw, use the provided bash script:

```bash
# Make the script executable (first time only)
chmod +x scripts/safeclaw.sh

# Log data
./scripts/safeclaw.sh log '{"action":"task_started","task":"deploy_app"}'

# List logs for an agent
./scripts/safeclaw.sh list 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266

# Get a specific log entry
./scripts/safeclaw.sh get 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 0

# Get agent summary
./scripts/safeclaw.sh summary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266

# Check service status
./scripts/safeclaw.sh status
```

The script automatically:
- Loads configuration from `.env` file
- Derives agent address from private key (if `cast` is available)
- Formats JSON output with colors
- Provides clear error messages

## Configuration

### Environment Variables

Create a `.env` file (use `.env.example` as template):

```bash
cp .env.example .env
```

Available configuration:
- `PORT` - HTTP API port (default: 12345)
- `HOST` - HTTP API host (default: 0.0.0.0)
- `NODE_ENV` - Environment (development/production)
- `START_ANVIL` - Set to 'true' to start embedded Anvil, 'false' to use external RPC (default: false)
- `ETHEREUM_RPC_URL` - Ethereum RPC endpoint (default: http://127.0.0.1:8545)
- `PRIVATE_KEY` - Private key for signing transactions (REQUIRED)
- `CONTRACT_ADDRESS` - Deployed contract address (automatically set after first deployment)

**Note**: SafeClaw will NOT automatically start Anvil unless `START_ANVIL=true` is set. This allows you to:
- Use an existing Anvil instance
- Connect to an external blockchain (testnet/mainnet)
- Run multiple SafeClaw instances against the same chain

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
3. Remove `CONTRACT_ADDRESS` from `.env` file to force redeployment

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

