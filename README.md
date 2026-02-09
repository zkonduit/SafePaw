# SafeClaw

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
│                   AgentTrace Sidecar                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  HTTP API (:3000)                                    │   │
│  │  POST /log        - submit action                    │   │
│  │  GET /entries     - list entries                     │   │
│  │  GET /verify      - verify chain integrity           │   │
│  │  GET /proof/:id   - get proof for entry             │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Embedded Anvil (:8545)                              │   │
│  │  - Auto-starts with sidecar                          │   │
│  │  - 1s block time                                     │   │
│  │  - Persists state to disk                            │   │
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
│   └── AgentLog.sol
├── sdk/
│   ├── python/
│   │   └── safeclaw/
│   │       └── __init__.py
│   └── typescript/
│       └── src/
│           └── index.ts
└── cli/
    └── safeclaw.sh     # Simple CLI wrapper
```

