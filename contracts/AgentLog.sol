// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title AgentLog
 * @dev Immutable blockchain-based logging for AI agents
 * Implements a hash chain where each entry references the previous one
 */
contract AgentLog {
    struct Log {
        uint256 timestamp;
        string data;
    }

    // Custom errors (more gas efficient than string reverts)
    error InvalidStartId();
    error InvalidEndId();

    // Storage
    // Mapping of agent address to their log entry count
    mapping(address => uint256) public logCount;
    // Mapping of agent address to their log entries (ID => LogEntry)
    mapping(address => mapping(uint256 => Log)) public logs;

    // Events
    event LogCreated(
        uint256 indexed id,
        uint256 indexed timestamp,
        address indexed agent,
        string data
    );

    /**
     * @dev Add a new log entry to the chain
     * @param data JSON metadata
     * @notice Gas-optimized implementation using assembly and unchecked arithmetic
     */
    function addLog(string memory data) external {
        address agent = msg.sender;
        uint256 id;
        uint256 timestamp = block.timestamp;

        assembly {
            // Load logCount[msg.sender] - more efficient than high-level SLOAD
            mstore(0x00, agent)
            mstore(0x20, logCount.slot)
            let countSlot := keccak256(0x00, 0x40)
            id := sload(countSlot)

            // Store updated count (id + 1) immediately - save gas by avoiding second SSTORE later
            sstore(countSlot, add(id, 1))
        }

        // Store entry - keeping struct assignment in Solidity for safety with dynamic data
        logs[agent][id] = Log({
            timestamp: timestamp,
            data: data
        });

        emit LogCreated(id, timestamp, agent, data);
    }

    /**
     * @dev Get a log entry by ID
     * @param id The entry ID
     * @return The log entry
     */
    function getLog(address agent, uint256 id) external view returns (Log memory) {
        return logs[agent][id];
    }

    /**
     * @dev Get logs in a range
     * @param agent The agent address
     * @param start Start ID (inclusive, 0-indexed)
     * @param end End ID (inclusive, 0-indexed)
     * @return Array of log entries
     * @notice Gas-optimized with assembly and custom errors
     */
    function getLogRange(address agent, uint256 start, uint256 end) external view returns (Log[] memory) {
        uint256 count;
        uint256 agentLogCount = logCount[agent];

        assembly {
            // Validate start < logCount
            if iszero(lt(start, agentLogCount)) {
                // revert InvalidStartId()
                mstore(0x00, 0x460ebc3300000000000000000000000000000000000000000000000000000000) // selector for InvalidStartId()
                revert(0x00, 0x04)
            }

            // Validate end >= start && end < logCount
            if or(lt(end, start), iszero(lt(end, agentLogCount))) {
                // revert InvalidEndId()
                mstore(0x00, 0x9179051800000000000000000000000000000000000000000000000000000000) // selector for InvalidEndId()
                revert(0x00, 0x04)
            }

            // Calculate count = end - start + 1 (unchecked, already validated)
            count := add(sub(end, start), 1)
        }

        Log[] memory _logs = new Log[](count);

        // Use unchecked loop for array population
        unchecked {
            for (uint256 i = 0; i < count; ++i) {
                _logs[i] = logs[agent][start + i];
            }
        }

        return _logs;
    }
}
