import express, { Request, Response, Router } from 'express';
import { ChainManager } from './chain';

export function createAPIRouter(
  chainManager: ChainManager
): Router {
  const router = express.Router();

  /**
   * POST /log
   * Submit a new log entry
   * Body: { data: string } where data is a JSON string
   */
  router.post('/log', async (req: Request, res: Response) => {
    try {
      const { data } = req.body;

      if (!data) {
        return res.status(400).json({
          error: 'Missing required field: data'
        });
      }

      // If data is an object, stringify it
      const dataStr = typeof data === 'string' ? data : JSON.stringify(data);

      const result = await chainManager.addLog(dataStr);

      res.json({
        success: true,
        id: result.id.toString(),
        timestamp: result.timestamp.toString(),
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
   * GET /entries/:agent
   * List all log entries for a specific agent (with optional pagination)
   * Query params: start (default: 0), limit (default: 100)
   */
  router.get('/entries/:agent', async (req: Request, res: Response) => {
    try {
      const agent = req.params.agent;
      const start = parseInt(req.query.start as string) || 0;
      const limit = parseInt(req.query.limit as string) || 100;

      const totalCount = await chainManager.getLogCount(agent);
      const total = Number(totalCount);

      if (total === 0) {
        return res.json({
          agent,
          entries: [],
          total: 0,
          start,
          limit
        });
      }

      const end = Math.min(start + limit - 1, total - 1);

      if (start >= total) {
        return res.status(400).json({
          error: 'Start index out of range'
        });
      }

      const entries = await chainManager.getLogRange(agent, start, end);

      // Format entries for JSON response
      const formattedEntries = entries.map((entry: any, index: number) => ({
        id: (start + index).toString(),
        timestamp: entry.timestamp.toString(),
        timestampISO: new Date(Number(entry.timestamp) * 1000).toISOString(),
        data: entry.data
      }));

      res.json({
        agent,
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
   * GET /entry/:agent/:id
   * Get a specific log entry by agent address and ID
   */
  router.get('/entry/:agent/:id', async (req: Request, res: Response) => {
    try {
      const agent = req.params.agent;
      const id = parseInt(req.params.id);

      if (isNaN(id) || id < 0) {
        return res.status(400).json({
          error: 'Invalid entry ID'
        });
      }

      const entry = await chainManager.getLog(agent, id);

      res.json({
        agent,
        id: id.toString(),
        timestamp: entry.timestamp.toString(),
        timestampISO: new Date(Number(entry.timestamp) * 1000).toISOString(),
        data: entry.data
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
   * GET /summary/:agent
   * Get a summary of logs for a specific agent
   */
  router.get('/summary/:agent', async (req: Request, res: Response) => {
    try {
      const agent = req.params.agent;
      const totalEntries = await chainManager.getLogCount(agent);

      let lastEntry = null;
      if (totalEntries > 0n) {
        const lastLog = await chainManager.getLog(agent, Number(totalEntries) - 1);
        lastEntry = {
          id: (Number(totalEntries) - 1).toString(),
          timestamp: lastLog.timestamp.toString(),
          timestampISO: new Date(Number(lastLog.timestamp) * 1000).toISOString(),
          data: lastLog.data
        };
      }

      res.json({
        agent,
        totalEntries: totalEntries.toString(),
        lastEntry,
        contractAddress: chainManager.getContractAddress()
      });
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
  router.get('/status', async (_req: Request, res: Response) => {
    try {
      const contractAddress = chainManager.getContractAddress();

      res.json({
        status: 'running',
        contractAddress,
        rpcUrl: process.env.ETHEREUM_RPC_URL || 'http://127.0.0.1:8545'
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
