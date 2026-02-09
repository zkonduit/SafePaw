# SafeClaw Dockerfile
# Build a containerized version of the SafeClaw AgentTrace Sidecar

FROM node:20-alpine AS builder

# Install Foundry (for Anvil and contract compilation)
RUN apk add --no-cache git curl bash
RUN curl -L https://foundry.paradigm.xyz | bash
ENV PATH="/root/.foundry/bin:${PATH}"
RUN foundryup

# Set working directory
WORKDIR /app

# Copy package files
COPY package*.json tsconfig.json ./

# Install dependencies
RUN npm ci

# Copy source code and contracts
COPY src ./src
COPY contracts ./contracts

# Compile TypeScript
RUN npm run build

# Compile Solidity contracts
RUN forge build

# Production stage
FROM node:20-alpine

# Install Foundry for Anvil
RUN apk add --no-cache git curl bash
RUN curl -L https://foundry.paradigm.xyz | bash
ENV PATH="/root/.foundry/bin:${PATH}"
RUN foundryup

WORKDIR /app

# Copy package files and install production dependencies only
COPY package*.json ./
RUN npm ci --only=production

# Copy built artifacts from builder
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/contracts ./contracts
COPY --from=builder /app/out ./out

# Expose ports
# 3000 - HTTP API
# 8545 - Anvil RPC
EXPOSE 3000 8545

# Create volume for persistent state
VOLUME ["/app/data"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD node -e "require('http').get('http://localhost:3000/status', (r) => { process.exit(r.statusCode === 200 ? 0 : 1); })"

# Start the sidecar
CMD ["node", "dist/index.js"]
