// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title AgentLog
 * @dev Immutable blockchain-based logging for AI agents
 * Implements a hash chain where each entry references the previous one
 */
contract AgentLog {
    struct LogEntry {
        uint256 id;
        uint256 timestamp;
        address agent;
        string action;
        string metadata;
        bytes32 previousHash;
        bytes32 currentHash;
    }

    // Storage
    uint256 private _entryCount;
    mapping(uint256 => LogEntry) private _entries;
    bytes32 private _lastHash;

    // Events
    event LogCreated(
        uint256 indexed id,
        address indexed agent,
        string action,
        bytes32 currentHash,
        bytes32 previousHash
    );

    /**
     * @dev Add a new log entry to the chain
     * @param action The action being logged
     * @param metadata Additional JSON metadata
     * @return id The ID of the created log entry
     */
    function addLog(string memory action, string memory metadata) external returns (uint256) {
        _entryCount++;
        uint256 id = _entryCount;
        uint256 timestamp = block.timestamp;
        address agent = msg.sender;
        bytes32 previousHash = _lastHash;

        // Create hash of current entry
        bytes32 currentHash = keccak256(
            abi.encodePacked(
                id,
                timestamp,
                agent,
                action,
                metadata,
                previousHash
            )
        );

        // Store entry
        _entries[id] = LogEntry({
            id: id,
            timestamp: timestamp,
            agent: agent,
            action: action,
            metadata: metadata,
            previousHash: previousHash,
            currentHash: currentHash
        });

        // Update last hash
        _lastHash = currentHash;

        emit LogCreated(id, agent, action, currentHash, previousHash);

        return id;
    }

    /**
     * @dev Get a log entry by ID
     * @param id The entry ID
     * @return The log entry
     */
    function getLog(uint256 id) external view returns (LogEntry memory) {
        require(id > 0 && id <= _entryCount, "Invalid log ID");
        return _entries[id];
    }

    /**
     * @dev Get the total number of log entries
     * @return The entry count
     */
    function getLogCount() external view returns (uint256) {
        return _entryCount;
    }

    /**
     * @dev Get the hash of the last entry
     * @return The last hash in the chain
     */
    function getLastHash() external view returns (bytes32) {
        return _lastHash;
    }

    /**
     * @dev Verify the integrity of a specific log entry
     * @param id The entry ID to verify
     * @return True if the entry's hash is valid
     */
    function verifyLog(uint256 id) external view returns (bool) {
        require(id > 0 && id <= _entryCount, "Invalid log ID");
        LogEntry memory entry = _entries[id];

        bytes32 computedHash = keccak256(
            abi.encodePacked(
                entry.id,
                entry.timestamp,
                entry.agent,
                entry.action,
                entry.metadata,
                entry.previousHash
            )
        );

        return computedHash == entry.currentHash;
    }

    /**
     * @dev Verify the entire chain from start to a specific entry
     * @param upToId The entry ID to verify up to (0 = verify all)
     * @return True if the chain is valid
     */
    function verifyChain(uint256 upToId) external view returns (bool) {
        if (upToId == 0 || upToId > _entryCount) {
            upToId = _entryCount;
        }

        bytes32 expectedPreviousHash = bytes32(0);

        for (uint256 i = 1; i <= upToId; i++) {
            LogEntry memory entry = _entries[i];

            // Verify previous hash matches
            if (entry.previousHash != expectedPreviousHash) {
                return false;
            }

            // Verify current hash is correct
            bytes32 computedHash = keccak256(
                abi.encodePacked(
                    entry.id,
                    entry.timestamp,
                    entry.agent,
                    entry.action,
                    entry.metadata,
                    entry.previousHash
                )
            );

            if (computedHash != entry.currentHash) {
                return false;
            }

            expectedPreviousHash = entry.currentHash;
        }

        return true;
    }

    /**
     * @dev Get logs in a range
     * @param start Start ID (inclusive)
     * @param end End ID (inclusive)
     * @return Array of log entries
     */
    function getLogRange(uint256 start, uint256 end) external view returns (LogEntry[] memory) {
        require(start > 0 && start <= _entryCount, "Invalid start ID");
        require(end >= start && end <= _entryCount, "Invalid end ID");

        uint256 count = end - start + 1;
        LogEntry[] memory logs = new LogEntry[](count);

        for (uint256 i = 0; i < count; i++) {
            logs[i] = _entries[start + i];
        }

        return logs;
    }
}
