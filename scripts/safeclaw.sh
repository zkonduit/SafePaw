#!/bin/bash

# SafeClaw Agent Logger
# Simple script for AI agents to log actions to SafeClaw server

set -e

# Load environment variables from .env file
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

# Configuration
SAFECLAW_URL="${SAFECLAW_URL:-http://localhost:12345}"
# Prefer PUBLIC_KEY, fallback to deriving from PRIVATE_KEY
AGENT_ADDRESS="${AGENT_ADDRESS:-${PUBLIC_KEY:-}}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print usage
usage() {
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  log <data>                   Log data to SafeClaw (data should be a JSON string)"
    echo "  get <agent> <id>             Get a specific log entry"
    echo "  list [agent] [start] [limit] List log entries (agent defaults to PUBLIC_KEY)"
    echo "  summary [agent]              Get summary (agent defaults to PUBLIC_KEY)"
    echo "  status                       Get SafeClaw service status"
    echo ""
    echo "Environment Variables:"
    echo "  SAFECLAW_URL           SafeClaw server URL (default: http://localhost:12345)"
    echo "  AGENT_ADDRESS          Agent wallet address (optional, overrides PUBLIC_KEY)"
    echo "  PUBLIC_KEY             Agent public address (used if AGENT_ADDRESS not set)"
    echo "  PRIVATE_KEY            Private key to derive address from (fallback)"
    echo ""
    echo "Examples:"
    echo "  $0 log '{\"action\":\"task_started\",\"task\":\"deploy_app\"}'"
    echo "  $0 list                                              # Uses PUBLIC_KEY from .env"
    echo "  $0 list 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266   # Explicit agent address"
    echo "  $0 get 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 0"
    echo "  $0 summary                                           # Uses PUBLIC_KEY from .env"
    echo "  $0 summary 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
    exit 1
}

# Function to derive address from private key
derive_address() {
    if [ -z "$PRIVATE_KEY" ]; then
        echo -e "${RED}Error: PRIVATE_KEY not found in .env file${NC}" >&2
        exit 1
    fi

    # Use cast to derive address from private key (requires foundry)
    if command -v cast &> /dev/null; then
        AGENT_ADDRESS=$(cast wallet address "$PRIVATE_KEY" 2>/dev/null || echo "")
        if [ -z "$AGENT_ADDRESS" ]; then
            echo -e "${YELLOW}Warning: Could not derive address from private key${NC}" >&2
            AGENT_ADDRESS="unknown"
        fi
    else
        echo -e "${YELLOW}Warning: 'cast' command not found. Install Foundry to derive agent address automatically.${NC}" >&2
        AGENT_ADDRESS="unknown"
    fi
}

# Function to log data
log_data() {
    local data="$1"

    if [ -z "$data" ]; then
        echo -e "${RED}Error: Data is required${NC}" >&2
        usage
    fi

    # Derive agent address if not set (and PUBLIC_KEY wasn't provided)
    if [ -z "$AGENT_ADDRESS" ]; then
        derive_address
    fi

    echo -e "${YELLOW}Logging to SafeClaw...${NC}"

    # Escape the data properly - the data field expects a JSON string
    # Use jq to properly escape and stringify the input
    if command -v jq &> /dev/null; then
        escaped_data=$(echo "$data" | jq -Rs .)
        response=$(curl -s -X POST "$SAFECLAW_URL/log" \
            -H "Content-Type: application/json" \
            -d "{\"data\":$escaped_data}")
    else
        # Fallback: just pass data as-is (requires proper JSON string from user)
        response=$(curl -s -X POST "$SAFECLAW_URL/log" \
            -H "Content-Type: application/json" \
            -d "{\"data\":\"$data\"}")
    fi

    if echo "$response" | grep -q '"success":true'; then
        echo -e "${GREEN}✓ Log submitted successfully${NC}"
        echo "$response" | jq '.' 2>/dev/null || echo "$response"
    else
        echo -e "${RED}✗ Failed to submit log${NC}" >&2
        echo "$response" | jq '.' 2>/dev/null || echo "$response"
        exit 1
    fi
}

# Function to get a specific log entry
get_log() {
    local agent="$1"
    local id="$2"

    if [ -z "$agent" ] || [ -z "$id" ]; then
        echo -e "${RED}Error: Agent address and ID are required${NC}" >&2
        usage
    fi

    echo -e "${YELLOW}Fetching log entry ${id} for agent ${agent}...${NC}"

    response=$(curl -s "$SAFECLAW_URL/entry/$agent/$id")

    if echo "$response" | grep -q '"error"'; then
        echo -e "${RED}✗ Failed to fetch log entry${NC}" >&2
        echo "$response" | jq '.' 2>/dev/null || echo "$response"
        exit 1
    else
        echo -e "${GREEN}✓ Log entry retrieved${NC}"
        echo "$response" | jq '.' 2>/dev/null || echo "$response"
    fi
}

# Function to list log entries
list_logs() {
    local agent="$1"
    local start="${2:-0}"
    local limit="${3:-10}"

    # Use PUBLIC_KEY or derive from PRIVATE_KEY if agent not specified
    if [ -z "$agent" ]; then
        if [ -n "$AGENT_ADDRESS" ]; then
            agent="$AGENT_ADDRESS"
            echo -e "${YELLOW}Using agent address from environment: ${agent}${NC}"
        elif [ -n "$PUBLIC_KEY" ]; then
            agent="$PUBLIC_KEY"
            echo -e "${YELLOW}Using PUBLIC_KEY from environment: ${agent}${NC}"
        else
            derive_address
            agent="$AGENT_ADDRESS"
            echo -e "${YELLOW}Derived agent address: ${agent}${NC}"
        fi
    fi

    echo -e "${YELLOW}Fetching logs for agent ${agent}...${NC}"

    response=$(curl -s "$SAFECLAW_URL/entries/$agent?start=$start&limit=$limit")

    if echo "$response" | grep -q '"error"'; then
        echo -e "${RED}✗ Failed to fetch logs${NC}" >&2
        echo "$response" | jq '.' 2>/dev/null || echo "$response"
        exit 1
    else
        echo -e "${GREEN}✓ Logs retrieved${NC}"
        echo "$response" | jq '.' 2>/dev/null || echo "$response"
    fi
}

# Function to get summary
get_summary() {
    local agent="$1"

    # Use PUBLIC_KEY or derive from PRIVATE_KEY if agent not specified
    if [ -z "$agent" ]; then
        if [ -n "$AGENT_ADDRESS" ]; then
            agent="$AGENT_ADDRESS"
            echo -e "${YELLOW}Using agent address from environment: ${agent}${NC}"
        elif [ -n "$PUBLIC_KEY" ]; then
            agent="$PUBLIC_KEY"
            echo -e "${YELLOW}Using PUBLIC_KEY from environment: ${agent}${NC}"
        else
            derive_address
            agent="$AGENT_ADDRESS"
            echo -e "${YELLOW}Derived agent address: ${agent}${NC}"
        fi
    fi

    echo -e "${YELLOW}Fetching summary for agent ${agent}...${NC}"

    response=$(curl -s "$SAFECLAW_URL/summary/$agent")

    if echo "$response" | grep -q '"error"'; then
        echo -e "${RED}✗ Failed to fetch summary${NC}" >&2
        echo "$response" | jq '.' 2>/dev/null || echo "$response"
        exit 1
    else
        echo -e "${GREEN}✓ Summary retrieved${NC}"
        echo "$response" | jq '.' 2>/dev/null || echo "$response"
    fi
}

# Function to get service status
get_status() {
    echo -e "${YELLOW}Fetching SafeClaw status...${NC}"

    response=$(curl -s "$SAFECLAW_URL/status")

    if echo "$response" | grep -q '"status":"running"'; then
        echo -e "${GREEN}✓ SafeClaw is running${NC}"
        echo "$response" | jq '.' 2>/dev/null || echo "$response"
    else
        echo -e "${RED}✗ SafeClaw is not responding${NC}" >&2
        echo "$response" | jq '.' 2>/dev/null || echo "$response"
        exit 1
    fi
}

# Main command dispatcher
case "$1" in
    log)
        log_data "$2"
        ;;
    get)
        get_log "$2" "$3"
        ;;
    list)
        list_logs "$2" "$3" "$4"
        ;;
    summary)
        get_summary "$2"
        ;;
    status)
        get_status
        ;;
    *)
        usage
        ;;
esac
