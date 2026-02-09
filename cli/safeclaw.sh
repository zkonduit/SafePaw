#!/bin/bash

# SafeClaw CLI Wrapper
# Simple command-line interface for interacting with SafeClaw AgentTrace Sidecar

SAFECLAW_URL="${SAFECLAW_URL:-http://localhost:3000}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
error() {
    echo -e "${RED}Error: $1${NC}" >&2
    exit 1
}

success() {
    echo -e "${GREEN}$1${NC}"
}

info() {
    echo -e "${YELLOW}$1${NC}"
}

# Check if jq is installed
check_jq() {
    if ! command -v jq &> /dev/null; then
        error "jq is required but not installed. Install it with: brew install jq (macOS) or apt-get install jq (Linux)"
    fi
}

# Check if curl is installed
check_curl() {
    if ! command -v curl &> /dev/null; then
        error "curl is required but not installed."
    fi
}

# Show usage
usage() {
    cat << EOF
SafeClaw CLI - AI Agent Observability and Provable Execution

Usage: safeclaw <command> [options]

Commands:
    log <action> [metadata]     Submit a new log entry
    entries [start] [limit]     List log entries (default: start=1, limit=100)
    entry <id>                  Get a specific log entry
    verify [upToId]             Verify chain integrity
    verify-entry <id>           Verify a specific entry
    proof <id>                  Get proof of execution for an entry
    health [interval] [tol]     Check agent health (default: interval=60, tolerance=30)
    tampering                   Detect potential tampering
    summary                     Get summary of logs and chain status
    status                      Get sidecar service status

Environment Variables:
    SAFECLAW_URL                Base URL of SafeClaw sidecar (default: http://localhost:3000)

Examples:
    safeclaw log "agent_started" '{"version":"1.0"}'
    safeclaw entries 1 10
    safeclaw entry 5
    safeclaw verify
    safeclaw health 60 30
    safeclaw summary

EOF
    exit 0
}

# Main command router
case "$1" in
    log)
        check_curl
        check_jq

        if [ -z "$2" ]; then
            error "Action is required. Usage: safeclaw log <action> [metadata]"
        fi

        ACTION="$2"
        METADATA="${3:-{}}"

        info "Logging action: $ACTION"

        RESPONSE=$(curl -s -X POST "$SAFECLAW_URL/log" \
            -H "Content-Type: application/json" \
            -d "{\"action\":\"$ACTION\",\"metadata\":$METADATA}")

        if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
            ID=$(echo "$RESPONSE" | jq -r '.id')
            HASH=$(echo "$RESPONSE" | jq -r '.hash')
            success "✓ Logged successfully"
            echo "  ID: $ID"
            echo "  Hash: $HASH"
        else
            error "Failed to log: $(echo "$RESPONSE" | jq -r '.error // .message // "Unknown error"')"
        fi
        ;;

    entries)
        check_curl
        check_jq

        START="${2:-1}"
        LIMIT="${3:-100}"

        info "Fetching entries (start=$START, limit=$LIMIT)"

        RESPONSE=$(curl -s "$SAFECLAW_URL/entries?start=$START&limit=$LIMIT")

        if echo "$RESPONSE" | jq -e '.entries' > /dev/null 2>&1; then
            TOTAL=$(echo "$RESPONSE" | jq -r '.total')
            success "✓ Found $TOTAL total entries"
            echo "$RESPONSE" | jq '.entries[] | "[\(.id)] \(.timestampISO) - \(.action)"' -r
        else
            error "Failed to fetch entries: $(echo "$RESPONSE" | jq -r '.error // .message // "Unknown error"')"
        fi
        ;;

    entry)
        check_curl
        check_jq

        if [ -z "$2" ]; then
            error "Entry ID is required. Usage: safeclaw entry <id>"
        fi

        ID="$2"

        info "Fetching entry $ID"

        RESPONSE=$(curl -s "$SAFECLAW_URL/entry/$ID")

        if echo "$RESPONSE" | jq -e '.id' > /dev/null 2>&1; then
            success "✓ Entry found"
            echo "$RESPONSE" | jq '.'
        else
            error "Failed to fetch entry: $(echo "$RESPONSE" | jq -r '.error // .message // "Unknown error"')"
        fi
        ;;

    verify)
        check_curl
        check_jq

        UP_TO_ID="${2:-}"
        URL="$SAFECLAW_URL/verify"

        if [ -n "$UP_TO_ID" ]; then
            URL="$URL?upToId=$UP_TO_ID"
            info "Verifying chain up to entry $UP_TO_ID"
        else
            info "Verifying entire chain"
        fi

        RESPONSE=$(curl -s "$URL")

        if echo "$RESPONSE" | jq -e '.valid' > /dev/null 2>&1; then
            VALID=$(echo "$RESPONSE" | jq -r '.valid')
            COUNT=$(echo "$RESPONSE" | jq -r '.entryCount')

            if [ "$VALID" = "true" ]; then
                success "✓ Chain is valid ($COUNT entries)"
            else
                error "✗ Chain verification failed ($COUNT entries)"
            fi

            echo "$RESPONSE" | jq '.'
        else
            error "Failed to verify: $(echo "$RESPONSE" | jq -r '.error // .message // "Unknown error"')"
        fi
        ;;

    verify-entry)
        check_curl
        check_jq

        if [ -z "$2" ]; then
            error "Entry ID is required. Usage: safeclaw verify-entry <id>"
        fi

        ID="$2"

        info "Verifying entry $ID"

        RESPONSE=$(curl -s "$SAFECLAW_URL/verify/$ID")

        if echo "$RESPONSE" | jq -e '.valid' > /dev/null 2>&1; then
            VALID=$(echo "$RESPONSE" | jq -r '.valid')

            if [ "$VALID" = "true" ]; then
                success "✓ Entry is valid"
            else
                error "✗ Entry verification failed"
            fi

            echo "$RESPONSE" | jq '.'
        else
            error "Failed to verify entry: $(echo "$RESPONSE" | jq -r '.error // .message // "Unknown error"')"
        fi
        ;;

    proof)
        check_curl
        check_jq

        if [ -z "$2" ]; then
            error "Entry ID is required. Usage: safeclaw proof <id>"
        fi

        ID="$2"

        info "Generating proof for entry $ID"

        RESPONSE=$(curl -s "$SAFECLAW_URL/proof/$ID")

        if echo "$RESPONSE" | jq -e '.entryId' > /dev/null 2>&1; then
            success "✓ Proof generated"
            echo "$RESPONSE" | jq '.'
        else
            error "Failed to generate proof: $(echo "$RESPONSE" | jq -r '.error // .message // "Unknown error"')"
        fi
        ;;

    health)
        check_curl
        check_jq

        INTERVAL="${2:-60}"
        TOLERANCE="${3:-30}"

        info "Checking agent health (interval=$INTERVAL, tolerance=$TOLERANCE)"

        RESPONSE=$(curl -s "$SAFECLAW_URL/health?expectedInterval=$INTERVAL&tolerance=$TOLERANCE")

        if echo "$RESPONSE" | jq -e '.healthy' > /dev/null 2>&1; then
            HEALTHY=$(echo "$RESPONSE" | jq -r '.healthy')

            if [ "$HEALTHY" = "true" ]; then
                success "✓ Agent is healthy"
            else
                error "✗ Agent health check failed"
            fi

            echo "$RESPONSE" | jq '.'
        else
            error "Failed to check health: $(echo "$RESPONSE" | jq -r '.error // .message // "Unknown error"')"
        fi
        ;;

    tampering)
        check_curl
        check_jq

        info "Detecting tampering"

        RESPONSE=$(curl -s "$SAFECLAW_URL/tampering")

        if echo "$RESPONSE" | jq -e '.tampered' > /dev/null 2>&1; then
            TAMPERED=$(echo "$RESPONSE" | jq -r '.tampered')

            if [ "$TAMPERED" = "false" ]; then
                success "✓ No tampering detected"
            else
                error "✗ Tampering detected!"
            fi

            echo "$RESPONSE" | jq '.'
        else
            error "Failed to detect tampering: $(echo "$RESPONSE" | jq -r '.error // .message // "Unknown error"')"
        fi
        ;;

    summary)
        check_curl
        check_jq

        info "Fetching summary"

        RESPONSE=$(curl -s "$SAFECLAW_URL/summary")

        if echo "$RESPONSE" | jq -e '.totalEntries' > /dev/null 2>&1; then
            success "✓ Summary retrieved"
            echo "$RESPONSE" | jq '.'
        else
            error "Failed to get summary: $(echo "$RESPONSE" | jq -r '.error // .message // "Unknown error"')"
        fi
        ;;

    status)
        check_curl
        check_jq

        info "Fetching service status"

        RESPONSE=$(curl -s "$SAFECLAW_URL/status")

        if echo "$RESPONSE" | jq -e '.status' > /dev/null 2>&1; then
            success "✓ Service is running"
            echo "$RESPONSE" | jq '.'
        else
            error "Failed to get status: $(echo "$RESPONSE" | jq -r '.error // .message // "Unknown error"')"
        fi
        ;;

    help|--help|-h|"")
        usage
        ;;

    *)
        error "Unknown command: $1. Run 'safeclaw help' for usage information."
        ;;
esac
