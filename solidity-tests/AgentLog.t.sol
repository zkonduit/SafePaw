// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../contracts/AgentLog.sol";

contract AgentLogTest is Test {
    AgentLog public agentLog;

    address public agent1 = address(0x1);
    address public agent2 = address(0x2);

    event LogCreated(
        uint256 indexed id,
        uint256 indexed timestamp,
        address indexed agent,
        string data
    );

    function setUp() public {
        agentLog = new AgentLog();
    }

    /* ========== BASIC FUNCTIONALITY TESTS ========== */

    function test_AddFirstLog() public {
        vm.prank(agent1);

        string memory testData = '{"action":"test","status":"success"}';

        vm.expectEmit(true, true, true, true);
        emit LogCreated(0, block.timestamp, agent1, testData);

        agentLog.addLog(testData);

        // Verify logCount
        assertEq(agentLog.logCount(agent1), 1, "Log count should be 1");

        // Verify log entry
        AgentLog.Log memory log = agentLog.getLog(agent1, 0);
        assertEq(log.timestamp, block.timestamp, "Timestamp should match");
        assertEq(log.data, testData, "Data should match");
    }

    function test_AddMultipleLogs() public {
        vm.startPrank(agent1);

        for (uint256 i = 0; i < 5; i++) {
            string memory testData = string(abi.encodePacked('{"id":', vm.toString(i), '}'));
            agentLog.addLog(testData);
        }

        vm.stopPrank();

        // Verify final count
        assertEq(agentLog.logCount(agent1), 5, "Log count should be 5");

        // Verify each log
        for (uint256 i = 0; i < 5; i++) {
            AgentLog.Log memory log = agentLog.getLog(agent1, i);
            string memory expected = string(abi.encodePacked('{"id":', vm.toString(i), '}'));
            assertEq(log.data, expected, "Log data should match");
        }
    }

    function test_MultipleAgentsIndependentLogs() public {
        // Agent 1 adds 3 logs
        vm.startPrank(agent1);
        agentLog.addLog("agent1_log1");
        agentLog.addLog("agent1_log2");
        agentLog.addLog("agent1_log3");
        vm.stopPrank();

        // Agent 2 adds 2 logs
        vm.startPrank(agent2);
        agentLog.addLog("agent2_log1");
        agentLog.addLog("agent2_log2");
        vm.stopPrank();

        // Verify counts
        assertEq(agentLog.logCount(agent1), 3, "Agent1 should have 3 logs");
        assertEq(agentLog.logCount(agent2), 2, "Agent2 should have 2 logs");

        // Verify independence
        AgentLog.Log memory agent1Log = agentLog.getLog(agent1, 0);
        AgentLog.Log memory agent2Log = agentLog.getLog(agent2, 0);
        assertEq(agent1Log.data, "agent1_log1");
        assertEq(agent2Log.data, "agent2_log1");
    }

    function test_LogIDsStartAtZero() public {
        vm.prank(agent1);
        agentLog.addLog("first");

        // First log should have ID 0
        AgentLog.Log memory log = agentLog.getLog(agent1, 0);
        assertEq(log.data, "first");
    }

    function test_LogTimestamps() public {
        vm.startPrank(agent1);

        uint256 timestamp1 = block.timestamp;
        agentLog.addLog("log1");

        // Advance time
        vm.warp(block.timestamp + 100);
        uint256 timestamp2 = block.timestamp;
        agentLog.addLog("log2");

        vm.stopPrank();

        // Verify timestamps
        AgentLog.Log memory log1 = agentLog.getLog(agent1, 0);
        AgentLog.Log memory log2 = agentLog.getLog(agent1, 1);

        assertEq(log1.timestamp, timestamp1);
        assertEq(log2.timestamp, timestamp2);
        assertGt(log2.timestamp, log1.timestamp, "Second log should be later");
    }

    /* ========== EDGE CASES ========== */

    function test_EmptyStringLog() public {
        vm.prank(agent1);
        agentLog.addLog("");

        AgentLog.Log memory log = agentLog.getLog(agent1, 0);
        assertEq(log.data, "", "Should accept empty string");
    }

    function test_VeryLongStringLog() public {
        // Create a long string (1000 characters)
        string memory longString = new string(1000);
        bytes memory longBytes = bytes(longString);
        for (uint256 i = 0; i < 1000; i++) {
            longBytes[i] = bytes1(uint8(65 + (i % 26))); // A-Z repeated
        }
        longString = string(longBytes);

        vm.prank(agent1);
        agentLog.addLog(longString);

        AgentLog.Log memory log = agentLog.getLog(agent1, 0);
        assertEq(bytes(log.data).length, 1000, "Should store long string");
    }

    function test_SpecialCharactersInLog() public {
        string memory specialData = '{"text":"Hello\\nWorld","emoji":"\xF0\x9F\x8E\x89"}';

        vm.prank(agent1);
        agentLog.addLog(specialData);

        AgentLog.Log memory log = agentLog.getLog(agent1, 0);
        assertEq(log.data, specialData, "Should handle special characters");
    }

    /* ========== GET LOG RANGE TESTS ========== */

    function test_GetLogRange() public {
        vm.startPrank(agent1);
        for (uint256 i = 0; i < 10; i++) {
            agentLog.addLog(vm.toString(i));
        }
        vm.stopPrank();

        // Get range 3-6 (inclusive, 0-indexed)
        AgentLog.Log[] memory logs = agentLog.getLogRange(agent1, 3, 6);

        assertEq(logs.length, 4, "Should return 4 logs");
        assertEq(logs[0].data, "3");
        assertEq(logs[3].data, "6");
    }

    function test_GetLogRangeSingleEntry() public {
        vm.startPrank(agent1);
        agentLog.addLog("only");
        vm.stopPrank();

        // 0-indexed: first and only log is at index 0
        AgentLog.Log[] memory logs = agentLog.getLogRange(agent1, 0, 0);
        assertEq(logs.length, 1);
        assertEq(logs[0].data, "only");
    }

    function test_GetLogRangeInvalidStart() public {
        vm.startPrank(agent1);
        agentLog.addLog("log1");
        vm.stopPrank();

        // logCount is 1, so valid index is only 0
        vm.expectRevert(AgentLog.InvalidStartId.selector);
        agentLog.getLogRange(agent1, 1, 1);

        vm.expectRevert(AgentLog.InvalidStartId.selector);
        agentLog.getLogRange(agent1, 2, 2);
    }

    function test_GetLogRangeInvalidEnd() public {
        vm.startPrank(agent1);
        agentLog.addLog("log1");
        agentLog.addLog("log2");
        vm.stopPrank();

        // logCount is 2, so valid indices are 0 and 1
        vm.expectRevert(AgentLog.InvalidEndId.selector);
        agentLog.getLogRange(agent1, 0, 2);

        vm.expectRevert(AgentLog.InvalidEndId.selector);
        agentLog.getLogRange(agent1, 1, 0);
    }

    /* ========== EVENT EMISSION TESTS ========== */

    function test_EventEmission() public {
        string memory testData = "test data";

        vm.prank(agent1);

        // Check all indexed and non-indexed parameters
        vm.expectEmit(true, true, true, true);
        emit LogCreated(0, block.timestamp, agent1, testData);

        agentLog.addLog(testData);
    }

    function test_MultipleEventEmissions() public {
        vm.startPrank(agent1);

        vm.expectEmit(true, true, true, true);
        emit LogCreated(0, block.timestamp, agent1, "first");
        agentLog.addLog("first");

        vm.expectEmit(true, true, true, true);
        emit LogCreated(1, block.timestamp, agent1, "second");
        agentLog.addLog("second");

        vm.stopPrank();
    }

    /* ========== GAS OPTIMIZATION TESTS ========== */

    function test_GasUsageAddLog() public {
        vm.prank(agent1);

        uint256 gasBefore = gasleft();
        agentLog.addLog('{"action":"test"}');
        uint256 gasUsed = gasBefore - gasleft();

        // Log gas usage for monitoring (should be lower with optimization)
        emit log_named_uint("Gas used for addLog", gasUsed);

        // Sanity check - should be reasonable
        assertLt(gasUsed, 150000, "Gas usage should be reasonable");
    }

    function test_GasUsageMultipleLogs() public {
        vm.startPrank(agent1);

        uint256 totalGas = 0;
        for (uint256 i = 0; i < 10; i++) {
            uint256 gasBefore = gasleft();
            agentLog.addLog(vm.toString(i));
            totalGas += gasBefore - gasleft();
        }

        vm.stopPrank();

        emit log_named_uint("Average gas per log (10 logs)", totalGas / 10);
    }

    /* ========== FUZZ TESTS ========== */

    function testFuzz_AddLog(string memory data) public {
        vm.assume(bytes(data).length < 10000); // Reasonable size limit

        vm.prank(agent1);
        agentLog.addLog(data);

        assertEq(agentLog.logCount(agent1), 1);
        AgentLog.Log memory log = agentLog.getLog(agent1, 0);
        assertEq(log.data, data);
    }

    function testFuzz_MultipleAgents(address agent, string memory data) public {
        vm.assume(agent != address(0));
        vm.assume(bytes(data).length < 1000);

        vm.prank(agent);
        agentLog.addLog(data);

        assertEq(agentLog.logCount(agent), 1);
        AgentLog.Log memory log = agentLog.getLog(agent, 0);
        assertEq(log.data, data);
    }

    function testFuzz_SequentialLogs(uint8 numLogs) public {
        vm.assume(numLogs > 0 && numLogs <= 50); // Reasonable range

        vm.startPrank(agent1);
        for (uint256 i = 0; i < numLogs; i++) {
            agentLog.addLog(vm.toString(i));
        }
        vm.stopPrank();

        assertEq(agentLog.logCount(agent1), numLogs);
    }

    /* ========== STRESS TESTS ========== */

    function test_AddManyLogs() public {
        vm.startPrank(agent1);

        uint256 iterations = 100;
        for (uint256 i = 0; i < iterations; i++) {
            agentLog.addLog(vm.toString(i));
        }

        vm.stopPrank();

        assertEq(agentLog.logCount(agent1), iterations);

        // Verify first and last
        AgentLog.Log memory firstLog = agentLog.getLog(agent1, 0);
        AgentLog.Log memory lastLog = agentLog.getLog(agent1, iterations - 1);
        assertEq(firstLog.data, "0");
        assertEq(lastLog.data, vm.toString(iterations - 1));
    }

    /* ========== INTEGRATION TESTS ========== */

    function test_CompleteWorkflow() public {
        // Scenario: Multiple agents logging various actions

        // Agent 1: Registration and first action
        vm.startPrank(agent1);
        agentLog.addLog('{"event":"registered","timestamp":1000}');
        agentLog.addLog('{"event":"first_action","result":"success"}');
        vm.stopPrank();

        // Agent 2: Different actions
        vm.startPrank(agent2);
        agentLog.addLog('{"event":"registered","timestamp":2000}');
        agentLog.addLog('{"event":"task_started","task_id":123}');
        agentLog.addLog('{"event":"task_completed","task_id":123}');
        vm.stopPrank();

        // Verify final states
        assertEq(agentLog.logCount(agent1), 2);
        assertEq(agentLog.logCount(agent2), 3);

        // Verify log ranges (0-indexed)
        AgentLog.Log[] memory agent2Logs = agentLog.getLogRange(agent2, 0, 2);
        assertEq(agent2Logs.length, 3);
    }
}
