import express from 'express';
import cors from 'cors';
import morgan from 'morgan';
import dotenv from 'dotenv';
import { ChainManager } from './chain';
import { VerificationService } from './verify';
import { createAPIRouter } from './api';

dotenv.config();

const PORT = process.env.PORT || 3000;
const HOST = process.env.HOST || '0.0.0.0';

async function main() {
  console.log('SafeClaw AgentTrace Sidecar');
  console.log('============================\n');

  // Initialize chain manager
  const chainManager = new ChainManager();

  try {
    // Start Anvil
    await chainManager.startAnvil();
    console.log('✓ Anvil started successfully\n');

    // Initialize provider and wallet
    await chainManager.initProvider();
    console.log('✓ Provider initialized\n');

    // Check if contract already exists
    const existingAddress = ChainManager.loadContractAddress();
    if (existingAddress) {
      console.log('Found existing contract at:', existingAddress);
      try {
        await chainManager.loadContract(existingAddress);
        console.log('✓ Contract loaded successfully\n');
      } catch (error) {
        console.log('Failed to load existing contract, deploying new one...');
        await chainManager.deployContract();
        console.log('✓ Contract deployed successfully\n');
      }
    } else {
      // Deploy contract
      await chainManager.deployContract();
      console.log('✓ Contract deployed successfully\n');
    }

    // Initialize verification service
    const verificationService = new VerificationService(chainManager);

    // Create Express app
    const app = express();

    // Middleware
    app.use(cors());
    app.use(morgan('combined'));
    app.use(express.json());

    // Health check endpoint
    app.get('/', (req, res) => {
      res.json({
        service: 'SafeClaw AgentTrace Sidecar',
        version: '1.0.0',
        status: 'running',
        contractAddress: chainManager.getContractAddress(),
        anvilRPC: 'http://127.0.0.1:8545',
        endpoints: {
          log: 'POST /log',
          entries: 'GET /entries',
          entry: 'GET /entry/:id',
          verify: 'GET /verify',
          verifyEntry: 'GET /verify/:id',
          proof: 'GET /proof/:id',
          health: 'GET /health',
          tampering: 'GET /tampering',
          summary: 'GET /summary',
          status: 'GET /status'
        }
      });
    });

    // API routes
    app.use('/', createAPIRouter(chainManager, verificationService));

    // Error handling middleware
    app.use((err: any, req: express.Request, res: express.Response, next: express.NextFunction) => {
      console.error('Unhandled error:', err);
      res.status(500).json({
        error: 'Internal server error',
        message: err.message
      });
    });

    // Start server
    const server = app.listen(PORT, () => {
      console.log(`✓ HTTP API listening on http://${HOST}:${PORT}\n`);
      console.log('SafeClaw is ready to receive logs!');
      console.log('\nExample usage:');
      console.log(`  curl -X POST http://localhost:${PORT}/log \\`);
      console.log(`    -H "Content-Type: application/json" \\`);
      console.log(`    -d '{"action":"agent_started","metadata":{"version":"1.0"}}'`);
      console.log();
    });

    // Graceful shutdown
    const shutdown = async () => {
      console.log('\nShutting down...');
      server.close();
      await chainManager.stop();
      process.exit(0);
    };

    process.on('SIGINT', shutdown);
    process.on('SIGTERM', shutdown);
  } catch (error) {
    console.error('Failed to start SafeClaw:', error);
    await chainManager.stop();
    process.exit(1);
  }
}

// Handle uncaught errors
process.on('uncaughtException', (error) => {
  console.error('Uncaught exception:', error);
  process.exit(1);
});

process.on('unhandledRejection', (reason, promise) => {
  console.error('Unhandled rejection at:', promise, 'reason:', reason);
  process.exit(1);
});

// Start the application
main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
