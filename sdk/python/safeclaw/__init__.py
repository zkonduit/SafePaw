"""
SafeClaw Python SDK

A client library for interacting with the SafeClaw AgentTrace Sidecar.
Provides observability and provable execution for AI agents.
"""

import requests
from typing import Optional, Dict, Any, List
from datetime import datetime


class SafeClawClient:
    """Client for interacting with SafeClaw AgentTrace Sidecar"""

    def __init__(self, base_url: str = "http://localhost:3000"):
        """
        Initialize the SafeClaw client.

        Args:
            base_url: The base URL of the SafeClaw sidecar (default: http://localhost:3000)
        """
        self.base_url = base_url.rstrip("/")
        self.session = requests.Session()
        self.session.headers.update({"Content-Type": "application/json"})

    def log(self, action: str, metadata: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """
        Submit a new log entry.

        Args:
            action: The action being logged
            metadata: Additional metadata (optional)

        Returns:
            Dict containing the log entry details (id, hash, txHash)

        Raises:
            requests.HTTPError: If the request fails
        """
        payload = {"action": action, "metadata": metadata or {}}

        response = self.session.post(f"{self.base_url}/log", json=payload)
        response.raise_for_status()

        return response.json()

    def get_entries(
        self, start: int = 1, limit: int = 100
    ) -> Dict[str, Any]:
        """
        Get log entries with pagination.

        Args:
            start: Start index (1-based, default: 1)
            limit: Maximum number of entries to return (default: 100)

        Returns:
            Dict containing entries list and pagination info

        Raises:
            requests.HTTPError: If the request fails
        """
        params = {"start": start, "limit": limit}
        response = self.session.get(f"{self.base_url}/entries", params=params)
        response.raise_for_status()

        return response.json()

    def get_entry(self, entry_id: int) -> Dict[str, Any]:
        """
        Get a specific log entry by ID.

        Args:
            entry_id: The entry ID

        Returns:
            Dict containing the entry details

        Raises:
            requests.HTTPError: If the request fails
        """
        response = self.session.get(f"{self.base_url}/entry/{entry_id}")
        response.raise_for_status()

        return response.json()

    def verify_chain(self, up_to_id: Optional[int] = None) -> Dict[str, Any]:
        """
        Verify the integrity of the entire chain.

        Args:
            up_to_id: Verify up to this entry ID (optional, default: all)

        Returns:
            Dict containing verification results

        Raises:
            requests.HTTPError: If the request fails
        """
        params = {"upToId": up_to_id} if up_to_id else {}
        response = self.session.get(f"{self.base_url}/verify", params=params)
        response.raise_for_status()

        return response.json()

    def verify_entry(self, entry_id: int) -> Dict[str, Any]:
        """
        Verify a specific log entry.

        Args:
            entry_id: The entry ID to verify

        Returns:
            Dict containing verification result

        Raises:
            requests.HTTPError: If the request fails
        """
        response = self.session.get(f"{self.base_url}/verify/{entry_id}")
        response.raise_for_status()

        return response.json()

    def get_proof(self, entry_id: int) -> Dict[str, Any]:
        """
        Get a proof of execution for a specific log entry.

        Args:
            entry_id: The entry ID

        Returns:
            Dict containing the proof details

        Raises:
            requests.HTTPError: If the request fails
        """
        response = self.session.get(f"{self.base_url}/proof/{entry_id}")
        response.raise_for_status()

        return response.json()

    def check_health(
        self, expected_interval: int = 60, tolerance: int = 30
    ) -> Dict[str, Any]:
        """
        Check the health of the agent (heartbeat verification).

        Args:
            expected_interval: Expected interval between logs in seconds (default: 60)
            tolerance: Tolerance in seconds (default: 30)

        Returns:
            Dict containing health status

        Raises:
            requests.HTTPError: If the request fails
        """
        params = {"expectedInterval": expected_interval, "tolerance": tolerance}
        response = self.session.get(f"{self.base_url}/health", params=params)
        response.raise_for_status()

        return response.json()

    def detect_tampering(self) -> Dict[str, Any]:
        """
        Detect potential tampering.

        Returns:
            Dict containing tampering detection results

        Raises:
            requests.HTTPError: If the request fails
        """
        response = self.session.get(f"{self.base_url}/tampering")
        response.raise_for_status()

        return response.json()

    def get_summary(self) -> Dict[str, Any]:
        """
        Get a summary of the logs and chain status.

        Returns:
            Dict containing summary information

        Raises:
            requests.HTTPError: If the request fails
        """
        response = self.session.get(f"{self.base_url}/summary")
        response.raise_for_status()

        return response.json()

    def get_status(self) -> Dict[str, Any]:
        """
        Get the status of the sidecar service.

        Returns:
            Dict containing service status

        Raises:
            requests.HTTPError: If the request fails
        """
        response = self.session.get(f"{self.base_url}/status")
        response.raise_for_status()

        return response.json()


# Convenience functions
def log_action(action: str, metadata: Optional[Dict[str, Any]] = None, base_url: str = "http://localhost:3000") -> Dict[str, Any]:
    """
    Convenience function to log an action without creating a client instance.

    Args:
        action: The action being logged
        metadata: Additional metadata (optional)
        base_url: The base URL of the SafeClaw sidecar

    Returns:
        Dict containing the log entry details
    """
    client = SafeClawClient(base_url)
    return client.log(action, metadata)


__all__ = ["SafeClawClient", "log_action"]
__version__ = "1.0.0"
