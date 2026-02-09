import express, { Request, Response, Router } from 'express';
import { ChainManager } from './chain';
import { VerificationService } from './verify';

export function createAPIRouter(
  chainManager: ChainManager,
  verificationService: VerificationService
): Router {
  const router = express.Router();

  /**
   * POST /log
   * Submit a new log entry
   * Body: { action: string, metadata: string }
   */
  router.post('/log', async (req: Request, res: Response) => {
    try {
      const { action, metadata } = req.body;

      if (!action) {
        return res.status(400).json({
          error: 'Missing required field: action'
        });
      }

      const metadataStr = metadata ? JSON.stringify(metadata) : '{}';

      const result = await chainManager.addLog(action, metadataStr);

      res.json({
        success: true,
        id: result.id.toString(),
        hash: result.hash,
        txHash: result.txHash
      });
    } catch (error: any) {
      console.error('Error adding log:', error);
      res.status(500).json({
        error: 'Failed to add log',
        message: error.message
      });
    }
  });

  /**
   * GET /entries
   * List all log entries (with optional pagination)
   * Query params: start (default: 1), limit (default: 100)
   */
  router.get('/entries', async (req: Request, res: Response) => {
    try {
      const start = parseInt(req.query.start as string) || 1;
      const limit = parseInt(req.query.limit as string) || 100;

      const totalCount = await chainManager.getLogCount();
      const total = Number(totalCount);

      if (total === 0) {
        return res.json({
          entries: [],
          total: 0,
          start,
          limit
        });
      }

      const end = Math.min(start + limit - 1, total);

      if (start > total) {
        return res.status(400).json({
          error: 'Start index out of range'
        });
      }

      const entries = await chainManager.getLogRange(start, end);

      // Format entries for JSON response
      const formattedEntries = entries.map((entry: any) => ({
        id: entry.id.toString(),
        timestamp: entry.timestamp.toString(),
        timestampISO: new Date(Number(entry.timestamp) * 1000).toISOString(),
        agent: entry.agent,
        action: entry.action,
        metadata: entry.metadata,
        previousHash: entry.previousHash,
        currentHash: entry.currentHash
      }));

      res.json({
        entries: formattedEntries,
        total,
        start,
        limit,
        returned: formattedEntries.length
      });
    } catch (error: any) {
      console.error('Error fetching entries:', error);
      res.status(500).json({
        error: 'Failed to fetch entries',
        message: error.message
      });
    }
  });

  /**
   * GET /entry/:id
   * Get a specific log entry by ID
   */
  router.get('/entry/:id', async (req: Request, res: Response) => {
    try {
      const id = parseInt(req.params.id);

      if (isNaN(id) || id < 1) {
        return res.status(400).json({
          error: 'Invalid entry ID'
        });
      }

      const entry = await chainManager.getLog(id);

      res.json({
        id: entry.id.toString(),
        timestamp: entry.timestamp.toString(),
        timestampISO: new Date(Number(entry.timestamp) * 1000).toISOString(),
        agent: entry.agent,
        action: entry.action,
        metadata: entry.metadata,
        previousHash: entry.previousHash,
        currentHash: entry.currentHash
      });
    } catch (error: any) {
      console.error('Error fetching entry:', error);
      res.status(500).json({
        error: 'Failed to fetch entry',
        message: error.message
      });
    }
  });

  /**
   * GET /verify
   * Verify the integrity of the entire chain
   * Query params: upToId (optional)
   */
  router.get('/verify', async (req: Request, res: Response) => {
    try {
      const upToId = req.query.upToId ? parseInt(req.query.upToId as string) : undefined;

      const result = await verificationService.verifyChainIntegrity(upToId);

      res.json({
        valid: result.valid,
        entryCount: result.entryCount,
        lastHash: result.lastHash,
        errors: result.errors,
        verifiedUpTo: upToId || result.entryCount
      });
    } catch (error: any) {
      console.error('Error verifying chain:', error);
      res.status(500).json({
        error: 'Failed to verify chain',
        message: error.message
      });
    }
  });

  /**
   * GET /verify/:id
   * Verify a specific log entry
   */
  router.get('/verify/:id', async (req: Request, res: Response) => {
    try {
      const id = parseInt(req.params.id);

      if (isNaN(id) || id < 1) {
        return res.status(400).json({
          error: 'Invalid entry ID'
        });
      }

      const result = await verificationService.verifyEntry(id);

      res.json({
        id,
        valid: result.valid,
        error: result.error
      });
    } catch (error: any) {
      console.error('Error verifying entry:', error);
      res.status(500).json({
        error: 'Failed to verify entry',
        message: error.message
      });
    }
  });

  /**
   * GET /proof/:id
   * Get a proof of execution for a specific log entry
   */
  router.get('/proof/:id', async (req: Request, res: Response) => {
    try {
      const id = parseInt(req.params.id);

      if (isNaN(id) || id < 1) {
        return res.status(400).json({
          error: 'Invalid entry ID'
        });
      }

      const proof = await verificationService.generateProof(id);

      res.json({
        entryId: proof.entryId.toString(),
        timestamp: proof.timestamp.toString(),
        timestampISO: new Date(Number(proof.timestamp) * 1000).toISOString(),
        action: proof.action,
        metadata: proof.metadata,
        currentHash: proof.currentHash,
        previousHash: proof.previousHash,
        chainValid: proof.chainValid,
        blockNumber: proof.blockNumber?.toString() || null,
        transactionHash: proof.transactionHash
      });
    } catch (error: any) {
      console.error('Error generating proof:', error);
      res.status(500).json({
        error: 'Failed to generate proof',
        message: error.message
      });
    }
  });

  /**
   * GET /health
   * Check the health of the agent (heartbeat verification)
   * Query params: expectedInterval (seconds, default: 60), tolerance (seconds, default: 30)
   */
  router.get('/health', async (req: Request, res: Response) => {
    try {
      const expectedInterval = parseInt(req.query.expectedInterval as string) || 60;
      const tolerance = parseInt(req.query.tolerance as string) || 30;

      const heartbeat = await verificationService.checkHeartbeat(expectedInterval, tolerance);

      res.json({
        healthy: heartbeat.healthy,
        lastLogTime: heartbeat.lastLogTime?.toISOString() || null,
        timeSinceLastLog: heartbeat.timeSinceLastLog,
        gaps: heartbeat.gaps,
        expectedInterval,
        tolerance
      });
    } catch (error: any) {
      console.error('Error checking health:', error);
      res.status(500).json({
        error: 'Failed to check health',
        message: error.message
      });
    }
  });

  /**
   * GET /tampering
   * Detect potential tampering
   */
  router.get('/tampering', async (req: Request, res: Response) => {
    try {
      const result = await verificationService.detectTampering();

      res.json(result);
    } catch (error: any) {
      console.error('Error detecting tampering:', error);
      res.status(500).json({
        error: 'Failed to detect tampering',
        message: error.message
      });
    }
  });

  /**
   * GET /summary
   * Get a summary of the logs and chain status
   */
  router.get('/summary', async (req: Request, res: Response) => {
    try {
      const summary = await verificationService.getSummary();

      const formattedSummary = {
        totalEntries: summary.totalEntries,
        chainValid: summary.chainValid,
        lastHash: summary.lastHash,
        lastEntry: summary.lastEntry
          ? {
              id: summary.lastEntry.id.toString(),
              timestamp: summary.lastEntry.timestamp.toString(),
              timestampISO: new Date(Number(summary.lastEntry.timestamp) * 1000).toISOString(),
              agent: summary.lastEntry.agent,
              action: summary.lastEntry.action
            }
          : null,
        contractAddress: chainManager.getContractAddress()
      };

      res.json(formattedSummary);
    } catch (error: any) {
      console.error('Error getting summary:', error);
      res.status(500).json({
        error: 'Failed to get summary',
        message: error.message
      });
    }
  });

  /**
   * GET /status
   * Get the status of the sidecar service
   */
  router.get('/status', async (req: Request, res: Response) => {
    try {
      const contractAddress = chainManager.getContractAddress();
      const totalEntries = await chainManager.getLogCount();

      res.json({
        status: 'running',
        contractAddress,
        totalEntries: totalEntries.toString(),
        anvilRPC: 'http://127.0.0.1:8545'
      });
    } catch (error: any) {
      console.error('Error getting status:', error);
      res.status(500).json({
        error: 'Failed to get status',
        message: error.message
      });
    }
  });

  return router;
}
