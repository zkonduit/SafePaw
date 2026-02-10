#!/bin/bash

# SafeClaw Startup Script
# Checks dependencies and starts services with Docker Compose

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Print banner
echo ""
echo "================================================"
echo "  SafeClaw - AI Agent Observability"
echo "================================================"
echo ""

# Check for Docker
print_info "Checking for Docker..."
if ! command_exists docker; then
    print_error "Docker is not installed!"
    echo ""
    echo "Please install Docker from: https://docs.docker.com/get-docker/"
    exit 1
fi
print_success "Docker is installed ($(docker --version))"

# Check if Docker daemon is running
if ! docker info >/dev/null 2>&1; then
    print_error "Docker daemon is not running!"
    echo ""
    echo "Please start Docker and try again."
    exit 1
fi
print_success "Docker daemon is running"

# Check for Docker Compose
print_info "Checking for Docker Compose..."
if docker compose version >/dev/null 2>&1; then
    COMPOSE_CMD="docker compose"
    print_success "Docker Compose is installed ($(docker compose version))"
elif command_exists docker-compose; then
    COMPOSE_CMD="docker-compose"
    print_success "Docker Compose is installed ($(docker-compose --version))"
else
    print_error "Docker Compose is not installed!"
    echo ""
    echo "Please install Docker Compose from: https://docs.docker.com/compose/install/"
    exit 1
fi

# Check if .env file exists
print_info "Checking for .env file..."
if [ ! -f .env ]; then
    print_warning ".env file not found. Creating from .env.example..."
    if [ -f .env.example ]; then
        cp .env.example .env
        print_success ".env file created"
        print_warning "Please review and update .env file with your configuration"
    else
        print_error ".env.example not found!"
        exit 1
    fi
else
    print_success ".env file exists"
fi

# Check if contracts are compiled
print_info "Checking for compiled contracts..."
if [ ! -f out/AgentLog.sol/AgentLog.json ]; then
    print_warning "Contracts not compiled. Checking for Foundry..."
    if command_exists forge; then
        print_info "Compiling contracts with Foundry..."
        forge build
        print_success "Contracts compiled successfully"
    else
        print_warning "Foundry is not installed. Attempting automatic installation..."
        echo ""

        # Install Foundry using foundryup
        if command_exists curl; then
            print_info "Downloading and running foundryup installer..."
            curl -L https://foundry.paradigm.xyz | bash

            # Source the environment to get foundryup in PATH
            if [ -f "$HOME/.foundry/bin/foundryup" ]; then
                export PATH="$HOME/.foundry/bin:$PATH"
                print_info "Running foundryup to install forge and other tools..."
                foundryup

                # Verify installation
                if command_exists forge; then
                    print_success "Foundry installed successfully!"
                    print_info "Compiling contracts with Foundry..."
                    forge build
                    print_success "Contracts compiled successfully"
                else
                    print_error "Foundry installation failed. Please install manually."
                    echo ""
                    echo "Visit: https://book.getfoundry.sh/getting-started/installation"
                    echo "Or run: curl -L https://foundry.paradigm.xyz | bash && foundryup"
                    exit 1
                fi
            else
                print_error "Foundry installation failed. Please install manually."
                echo ""
                echo "Visit: https://book.getfoundry.sh/getting-started/installation"
                exit 1
            fi
        else
            print_error "curl is not installed! Cannot automatically install Foundry."
            echo ""
            echo "Please install curl or install Foundry manually from:"
            echo "https://book.getfoundry.sh/getting-started/installation"
            exit 1
        fi
    fi
else
    print_success "Contracts are compiled"
fi

# Install SafeClaw skill to OpenClaw
print_info "Installing SafeClaw skill to OpenClaw..."
SKILL_DIR="${HOME}/.openclaw/skills/safeclaw"
if [ -f SKILL.md ]; then
    # Create the skills directory if it doesn't exist
    mkdir -p "$SKILL_DIR"

    # Copy the skill file
    cp SKILL.md "$SKILL_DIR/SKILL.md"
    print_success "SafeClaw skill installed to $SKILL_DIR/SKILL.md"
else
    print_warning "SKILL.md not found. Skipping skill installation."
fi

# Parse command line arguments
DETACH=""
REBUILD=""
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--detach)
            DETACH="-d"
            shift
            ;;
        -r|--rebuild)
            REBUILD="--build"
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -d, --detach    Run services in detached mode (background)"
            echo "  -r, --rebuild   Rebuild Docker images before starting"
            echo "  -h, --help      Show this help message"
            echo ""
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Stop existing containers if running
print_info "Stopping any existing SafeClaw containers..."
$COMPOSE_CMD down 2>/dev/null || true

# Check if Anvil is already running
print_info "Checking for Anvil..."
ANVIL_RUNNING=false
if lsof -Pi :8545 -sTCP:LISTEN -t >/dev/null 2>&1; then
    ANVIL_RUNNING=true
    print_success "Anvil is already running on port 8545"
else
    print_info "Starting Anvil on host machine..."
    if command_exists anvil; then
        # Start Anvil in background
        nohup anvil --host 0.0.0.0 --port 8545 --block-time 5 --accounts 10 --balance 10000 > anvil.log 2>&1 &
        ANVIL_PID=$!
        echo $ANVIL_PID > .anvil.pid

        # Wait a moment for Anvil to start
        sleep 2

        if lsof -Pi :8545 -sTCP:LISTEN -t >/dev/null 2>&1; then
            print_success "Anvil started successfully (PID: $ANVIL_PID)"
            print_info "Anvil logs available at: anvil.log"
        else
            print_error "Failed to start Anvil. Check anvil.log for details."
            exit 1
        fi
    else
        print_error "Anvil (Foundry) is not installed!"
        echo ""
        echo "Please install Foundry first:"
        echo "  curl -L https://foundry.paradigm.xyz | bash"
        echo "  foundryup"
        exit 1
    fi
fi

# Start services
echo ""
echo "================================================"
echo "  Starting SafeClaw Services"
echo "================================================"
echo ""

if [ -n "$REBUILD" ]; then
    print_info "Building Docker images..."
fi

if [ -n "$DETACH" ]; then
    print_info "Starting services in detached mode..."
    $COMPOSE_CMD up $DETACH $REBUILD

    echo ""
    print_success "SafeClaw services started successfully!"
    echo ""
    echo "Services:"
    echo "  - Anvil:    http://localhost:8545"
    echo "  - SafeClaw: http://localhost:12345"
    echo ""
    echo "Useful commands:"
    echo "  - View logs:     $COMPOSE_CMD logs -f"
    echo "  - Stop services: $COMPOSE_CMD down"
    echo "  - View status:   $COMPOSE_CMD ps"
    echo "  - View Anvil logs: tail -f anvil.log"
    if [ "$ANVIL_RUNNING" = false ]; then
        echo "  - Stop Anvil:    kill \$(cat .anvil.pid)"
    fi
    echo ""
else
    print_info "Starting services (press Ctrl+C to stop)..."
    echo ""

    # Setup cleanup trap for non-detached mode
    cleanup() {
        echo ""
        print_info "Shutting down services..."
        $COMPOSE_CMD down
        if [ "$ANVIL_RUNNING" = false ] && [ -f .anvil.pid ]; then
            print_info "Stopping Anvil..."
            kill $(cat .anvil.pid) 2>/dev/null || true
            rm -f .anvil.pid
        fi
        exit 0
    }
    trap cleanup INT TERM

    $COMPOSE_CMD up $REBUILD
fi
