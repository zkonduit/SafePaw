#!/bin/bash

# SafeClaw Stop Script
# Stops all SafeClaw services including Anvil

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Print banner
echo ""
echo "================================================"
echo "  Stopping SafeClaw Services"
echo "================================================"
echo ""

# Detect Docker Compose command
if docker compose version >/dev/null 2>&1; then
    COMPOSE_CMD="docker compose"
elif command -v docker-compose >/dev/null 2>&1; then
    COMPOSE_CMD="docker-compose"
else
    print_error "Docker Compose not found!"
    exit 1
fi

# Stop Docker containers
print_info "Stopping SafeClaw containers..."
$COMPOSE_CMD down 2>/dev/null || true
print_success "SafeClaw containers stopped"

# Stop Anvil if running
if [ -f .anvil.pid ]; then
    ANVIL_PID=$(cat .anvil.pid)
    print_info "Stopping Anvil (PID: $ANVIL_PID)..."
    if kill "$ANVIL_PID" 2>/dev/null; then
        print_success "Anvil stopped"
    else
        print_info "Anvil process not found (may have already stopped)"
    fi
    rm -f .anvil.pid
else
    # Try to find and stop Anvil by port
    if lsof -Pi :8545 -sTCP:LISTEN -t >/dev/null 2>&1; then
        print_info "Found Anvil running on port 8545, stopping..."
        ANVIL_PID=$(lsof -Pi :8545 -sTCP:LISTEN -t)
        kill "$ANVIL_PID" 2>/dev/null || true
        print_success "Anvil stopped"
    else
        print_info "Anvil is not running"
    fi
fi

echo ""
print_success "All SafeClaw services stopped"
echo ""
