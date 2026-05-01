# Gas Relayer — Complete Usage Guide

> Production-grade ERC-2771 meta-transaction relayer in Rust.  
> Lets your dApp users submit transactions without ever holding ETH.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Prerequisites](#prerequisites)
3. [Installation & Build](#installation--build)
4. [Configuration](#configuration)
5. [Private Key Management](#private-key-management)
6. [CLI Reference](#cli-reference)
7. [Server Startup](#server-startup)
8. [API Reference](#api-reference)
9. [Integrating Your dApp](#integrating-your-dapp)
10. [Multi-Tenant Setup (Teams)](#multi-tenant-setup-teams)
11. [Docker & Deployment](#docker--deployment)
12. [Database Setup](#database-setup)
13. [Monitoring & Observability](#monitoring--observability)
14. [Security Hardening](#security-hardening)
15. [Troubleshooting](#troubleshooting)

---

## Quick Start

```bash
# 1. Clone and build
git clone https://github.com/yourteam/gas-relayer
cd gas-relayer
cargo build --release

# 2. Set environment
cp .env.example .env
# Edit .env with your RPC URL, forwarder address, private key

# 3. Run database migrations
psql $DATABASE_URL < migrations/001_initial.sql

# 4. Start the relayer
RUST_LOG=info ./target/release/gas-relayer

# 5. Verify it's running
curl http://localhost:8080/health
# {"status":"ok","version":"0.1.0"}
```

---

## Prerequisites

| Tool | Version | Purpose |
|---|---|---|
| Rust | ≥ 1.75 | Build the relayer |
| PostgreSQL | ≥ 14 | Persistent job storage |
| Redis | ≥ 7 | Rate limiting, queue overflow |
| An Ethereum RPC | — | Alchemy, Infura, or self-hosted |
| A funded wallet | — | Pays gas on behalf of users |
| MinimalForwarder | deployed | On-chain ERC-2771 contract |

### Deploy the MinimalForwarder

Before running the relayer, you need a `MinimalForwarder` contract deployed on each chain you want to support.

Using Foundry:

```bash
# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Deploy to a local Anvil node
anvil &

forge create contracts/MinimalForwarder.sol:MinimalForwarder \
  --rpc-url http://localhost:8545 \
  --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

# Note the deployed address — this goes in FORWARDER_ADDRESS
# Deployed to: 0x5FbDB2315678afecb367f032d93F642f64180aa3
```

Using Hardhat:

```bash
npx hardhat run scripts/deploy-forwarder.js --network mainnet
```

OpenZeppelin's pre-audited version (recommended for production):

```solidity
import "@openzeppelin/contracts/metatx/MinimalForwarder.sol";
```

---

## Installation & Build

### From Source

```bash
# Debug build (fast compile, slower runtime)
cargo build

# Release build (optimized — use this in production)
cargo build --release

# The binary is at:
./target/release/gas-relayer
```

### Run Tests

```bash
# Run all unit tests
cargo test

# Run with output visible
cargo test -- --nocapture

# Run a specific test
cargo test test_eip712_domain_separator

# Run tests in a specific module
cargo test crypto::eip712
```

### Check for Issues

```bash
# Lint
cargo clippy -- -D warnings

# Format check
cargo fmt --check

# Security audit (requires cargo-audit)
cargo install cargo-audit
cargo audit
```

---

## Configuration

Configuration is loaded in this priority order (highest wins):

```
Environment variables  >  config.yaml  >  compiled defaults
```

### Environment Variables

All environment variables can also be set with the `RELAYER__` prefix using double underscores as separators:

```bash
# These are equivalent:
PORT=9090
RELAYER__SERVER__PORT=9090
```

| Variable | Required | Default | Description |
|---|---|---|---|
| `CHAIN_ID` | Yes | `1` | Target blockchain chain ID |
| `RPC_URL` | Yes | `http://localhost:8545` | HTTP JSON-RPC endpoint |
| `FORWARDER_ADDRESS` | Yes | — | MinimalForwarder contract address |
| `RELAYER_PRIVATE_KEY` | Dev only | — | Hex private key (dev/testing only) |
| `DOMAIN_NAME` | Yes | `GasRelayer` | EIP-712 domain name |
| `DATABASE_URL` | Yes | — | PostgreSQL connection string |
| `REDIS_URL` | No | `redis://localhost:6379` | Redis connection string |
| `HOST` | No | `0.0.0.0` | Bind address |
| `PORT` | No | `8080` | HTTP port |
| `RUST_LOG` | No | `info` | Log level filter |

### config.yaml

Copy the provided `config.yaml` and edit for your environment. The file supports multiple chains:

```yaml
chains:
  - chain_id: 1
    name: "Ethereum Mainnet"
    rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY"
    forwarder_address: "0xYOUR_FORWARDER"
    domain_name: "MyDAppRelayer"
    domain_version: "1"
    max_gas_price_gwei: 150
    confirmations: 2
    confirmation_timeout_secs: 180
```

---

## Private Key Management

This is the most critical security decision in your deployment.

### Development — Environment Variable

```bash
# .env
RELAYER_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
```

Suitable for: local dev, CI/CD test environments only.

### Staging — AWS Secrets Manager

```bash
# Store the key
aws secretsmanager create-secret \
  --name "gas-relayer/staging/private-key" \
  --secret-string "0xYOUR_PRIVATE_KEY"

# In your startup script, fetch at runtime:
export RELAYER_PRIVATE_KEY=$(aws secretsmanager get-secret-value \
  --secret-id "gas-relayer/staging/private-key" \
  --query SecretString --output text)
```

### Production — AWS KMS (Recommended)

With AWS KMS, the private key **never leaves AWS**. The relayer calls KMS to sign each transaction digest remotely.

```bash
# Create a secp256k1 KMS key
aws kms create-key \
  --key-spec ECC_SECG_P256K1 \
  --key-usage SIGN_VERIFY \
  --description "Gas Relayer Mainnet Signing Key"

# Note the KeyId/ARN
# arn:aws:kms:us-east-1:123456789012:key/abc123

# Grant the relayer EC2/ECS role permission to sign
aws kms create-grant \
  --key-id arn:aws:kms:... \
  --grantee-principal arn:aws:iam::123456789012:role/gas-relayer-role \
  --operations Sign GetPublicKey
```

Set in config:
```yaml
relayer:
  key_provider: aws_kms
  key_reference: "arn:aws:kms:us-east-1:123456789012:key/abc123"
```

### Production — HashiCorp Vault Transit

```bash
# Enable Transit secrets engine
vault secrets enable transit

# Create a secp256k1 signing key for each team
vault write transit/keys/team-abc123-mainnet type=ecdsa-secp256k1

# The key_reference stored in the DB:
# "vault:transit/keys/team-abc123-mainnet"
```

### Multi-Tenant Key Isolation

Each team that integrates with your relayer gets their own funded wallet and their own key in KMS/Vault:

```
Team A → KMS key A → Wallet 0xAAA (funded with ETH for gas)
Team B → KMS key B → Wallet 0xBBB (funded with ETH for gas)
Team C → KMS key C → Wallet 0xCCC (funded with ETH for gas)
```

This isolation means:
- Team A's budget depletion doesn't affect Team B
- A compromised key only exposes that team's wallet
- You can revoke a team's key without touching others
- Gas billing is per-team (each funds their own wallet)

---

## CLI Reference

### Run the Server

```bash
# Basic startup
./target/release/gas-relayer

# With explicit env
CHAIN_ID=137 RPC_URL=https://polygon-rpc.com PORT=9090 ./target/release/gas-relayer

# With config file
./target/release/gas-relayer  # auto-loads config.yaml from current directory

# Debug logging
RUST_LOG=gas_relayer=debug,ethers=info ./target/release/gas-relayer
```

### Docker

```bash
# Build image
docker build -t gas-relayer:latest .

# Run container
docker run -d \
  --name gas-relayer \
  -p 8080:8080 \
  -e CHAIN_ID=1 \
  -e RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY" \
  -e FORWARDER_ADDRESS="0xYOUR_FORWARDER" \
  -e RELAYER_PRIVATE_KEY="0xYOUR_KEY" \
  -e DATABASE_URL="postgresql://..." \
  gas-relayer:latest

# Check logs
docker logs -f gas-relayer

# Health check
docker exec gas-relayer curl -s http://localhost:8080/health
```

### Docker Compose (Full Stack)

```bash
# Start everything (relayer + postgres + redis + local anvil node)
docker compose up -d

# View logs
docker compose logs -f relayer

# Stop
docker compose down

# Reset database
docker compose down -v  # removes volumes
docker compose up -d
```

### Database Migrations

```bash
# Apply migrations manually
psql $DATABASE_URL < migrations/001_initial.sql

# Using sqlx-cli
cargo install sqlx-cli
sqlx migrate run --database-url $DATABASE_URL

# Check migration status
sqlx migrate info --database-url $DATABASE_URL
```

---

## Server Startup

When the relayer starts, it logs its initialization sequence:

```
2024-01-15T10:00:00Z  INFO gas_relayer: 🚀 Gas Relayer starting version="0.1.0"
2024-01-15T10:00:00Z  INFO gas_relayer: connected to RPC rpc_url="https://..." chain_id=1
2024-01-15T10:00:00Z  INFO gas_relayer: relayer wallet loaded relayer=0xABC...
2024-01-15T10:00:00Z  INFO gas_relayer: EIP-712 verifier initialized domain_name="GasRelayer" forwarder=0x123... domain_separator=0xDEF...
2024-01-15T10:00:00Z  INFO gas_relayer: relay worker pool started (4 workers)
2024-01-15T10:00:00Z  INFO gas_relayer: starting HTTP server addr="0.0.0.0:8080"
```

The `domain_separator` logged at startup is important — it must match what your frontend computes for signatures to verify correctly.

---

## API Reference

Base URL: `http://localhost:8080`

### POST `/relay`

Submit a meta-transaction for gasless relaying.

**Request Body:**

```json
{
  "request": {
    "from":     "0xUSER_ADDRESS",
    "to":       "0xTARGET_CONTRACT",
    "value":    "0x0",
    "gas":      "0x186a0",
    "nonce":    "0x0",
    "deadline": "0x67890abc",
    "data":     "0xa9059cbb000000000000000000000000RECIPIENT0000000000000000000000000000000000000000000000000DE0B6B3A7640000"
  },
  "signature": "0x...(65 bytes, 130 hex chars)",
  "chainId": 1
}
```

**Fields:**

| Field | Type | Description |
|---|---|---|
| `request.from` | address | User's wallet address (signer) |
| `request.to` | address | Target contract to call |
| `request.value` | uint256 (hex) | ETH to forward (usually "0x0") |
| `request.gas` | uint256 (hex) | Gas limit for the inner call |
| `request.nonce` | uint256 (hex) | User's forwarder nonce (get from `/nonce`) |
| `request.deadline` | uint256 (hex) | Unix timestamp expiry (recommended) |
| `request.data` | bytes (hex) | ABI-encoded function call |
| `signature` | bytes (hex) | 65-byte EIP-712 signature |
| `chainId` | number | Target chain |

**Success Response (202 Accepted):**

```json
{
  "jobId": "550e8400-e29b-41d4-a716-446655440000",
  "status": "queued",
  "estimatedSeconds": 15
}
```

**Error Responses:**

```json
{ "error": "nonce mismatch: expected 1, got 0", "code": "NONCE_MISMATCH" }
{ "error": "signer mismatch: expected 0xAAA, recovered 0xBBB", "code": "INVALID_SIGNATURE" }
{ "error": "target contract 0x... is not in allowed list", "code": "POLICY_VIOLATION" }
{ "error": "request deadline has passed", "code": "INVALID_SIGNATURE" }
```

---

### GET `/relay/:jobId`

Poll the status of a submitted relay job.

```bash
curl http://localhost:8080/relay/550e8400-e29b-41d4-a716-446655440000
```

**Response:**

```json
{
  "jobId": "550e8400-e29b-41d4-a716-446655440000",
  "status": "confirmed",
  "txHash": "0xabc123...",
  "blockNumber": 19000000,
  "error": null
}
```

**Status values:** `pending` → `queued` → `processing` → `submitted` → `confirmed` / `failed` / `reverted`

---

### GET `/nonce/:chainId/:address`

Get the current forwarder nonce for a user address. Call this before building a `ForwardRequest`.

```bash
curl http://localhost:8080/nonce/1/0xUSER_ADDRESS
```

**Response:**

```json
{
  "address": "0xUSER_ADDRESS",
  "chainId": 1,
  "nonce": "0x5"
}
```

---

### GET `/domain/:chainId`

Get the EIP-712 domain parameters. Your frontend needs these to construct the typed data for signing.

```bash
curl http://localhost:8080/domain/1
```

**Response:**

```json
{
  "name": "GasRelayer",
  "version": "1",
  "chainId": 1,
  "verifyingContract": "0xFORWARDER_ADDRESS",
  "domainSeparator": "0xabc123..."
}
```

---

### GET `/health`

Liveness probe. Returns 200 if the server is running.

```bash
curl http://localhost:8080/health
# {"status":"ok","version":"0.1.0"}
```

---

## Integrating Your dApp

### Step 1 — Make Your Contract ERC-2771 Compatible

```solidity
import "@openzeppelin/contracts/metatx/ERC2771Context.sol";

contract MyToken is ERC20, ERC2771Context {
    constructor(address forwarder) ERC2771Context(forwarder) {}

    function transfer(address to, uint256 amount) public override returns (bool) {
        // Replace msg.sender with _msgSender() everywhere
        return _transfer(_msgSender(), to, amount);
    }
}
```

### Step 2 — Frontend Integration (JavaScript/TypeScript)

```typescript
import { ethers } from "ethers";

const RELAYER_URL = "https://your-relayer.com";
const FORWARDER_ADDRESS = "0xYOUR_FORWARDER";

async function sendGaslessTransaction(
  signer: ethers.Signer,
  targetContract: string,
  calldata: string
) {
  const userAddress = await signer.getAddress();
  const chainId = await signer.getChainId();

  // 1. Fetch nonce from relayer
  const nonceResp = await fetch(
    `${RELAYER_URL}/nonce/${chainId}/${userAddress}`
  );
  const { nonce } = await nonceResp.json();

  // 2. Fetch domain from relayer
  const domainResp = await fetch(`${RELAYER_URL}/domain/${chainId}`);
  const domain = await domainResp.json();

  // 3. Build ForwardRequest
  const request = {
    from: userAddress,
    to: targetContract,
    value: "0",
    gas: 150000,
    nonce: parseInt(nonce, 16),
    deadline: Math.floor(Date.now() / 1000) + 3600, // 1 hour from now
    data: calldata,
  };

  // 4. Sign with EIP-712 (eth_signTypedData_v4)
  const types = {
    ForwardRequest: [
      { name: "from", type: "address" },
      { name: "to", type: "address" },
      { name: "value", type: "uint256" },
      { name: "gas", type: "uint256" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
      { name: "data", type: "bytes" },
    ],
  };

  const eip712Domain = {
    name: domain.name,
    version: domain.version,
    chainId: domain.chainId,
    verifyingContract: domain.verifyingContract,
  };

  const signature = await signer._signTypedData(eip712Domain, types, request);

  // 5. Submit to relayer
  const relayResp = await fetch(`${RELAYER_URL}/relay`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      request: {
        ...request,
        value: "0x0",
        gas: ethers.utils.hexlify(request.gas),
        nonce: ethers.utils.hexlify(request.nonce),
        deadline: ethers.utils.hexlify(request.deadline),
      },
      signature,
      chainId,
    }),
  });

  const { jobId } = await relayResp.json();

  // 6. Poll for confirmation
  return pollJobStatus(jobId);
}

async function pollJobStatus(jobId: string): Promise<string> {
  while (true) {
    const resp = await fetch(`${RELAYER_URL}/relay/${jobId}`);
    const job = await resp.json();

    if (job.status === "confirmed") return job.txHash;
    if (job.status === "failed" || job.status === "reverted") {
      throw new Error(`Relay failed: ${job.error}`);
    }

    await new Promise((r) => setTimeout(r, 2000)); // poll every 2s
  }
}
```

### Step 3 — Encode Your Function Call

```typescript
const iface = new ethers.utils.Interface([
  "function transfer(address to, uint256 amount)"
]);

const calldata = iface.encodeFunctionData("transfer", [
  "0xRECIPIENT_ADDRESS",
  ethers.utils.parseUnits("100", 18) // 100 tokens
]);
```

---

## Multi-Tenant Setup (Teams)

Each team that wants to offer gasless transactions to their users:

**1. Creates a policy** (via your admin API or DB insert):

```sql
INSERT INTO teams (name, api_key_hash) VALUES ('Acme Games', sha256('their-api-key'));

INSERT INTO policies (
  team_id, name, chain_id, forwarder_address, relayer_address,
  allowed_targets, max_gas_per_request
) VALUES (
  'team-uuid',
  'Acme Games Policy',
  137,
  '0xFORWARDER',
  '0xACME_RELAYER_WALLET',  -- their funded wallet
  '{"type": "allowlist", "addresses": ["0xACME_TOKEN"]}',
  300000
);
```

**2. Funds their relayer wallet** (they send MATIC/ETH to their assigned wallet).

**3. Integrates the frontend SDK** with their API key in the `Authorization` header.

**Billing model options:**
- Teams fund their own wallet (most common — they control their spend)
- You charge teams a monthly fee and fund all wallets centrally
- Pay-as-you-go: charge teams per-relay in a stablecoin

---

## Docker & Deployment

### Build & Push

```bash
# Build
docker build -t gas-relayer:$(git rev-parse --short HEAD) .
docker tag gas-relayer:$(git rev-parse --short HEAD) yourregistry/gas-relayer:latest

# Push
docker push yourregistry/gas-relayer:latest
```

### Full Stack with Docker Compose

```bash
# Start all services
docker compose up -d

# Scale workers (if you refactor to separate worker process)
docker compose up -d --scale relayer=3

# View logs
docker compose logs -f

# Stop
docker compose down
```

### Production — Kubernetes (Example)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: gas-relayer
spec:
  replicas: 2
  template:
    spec:
      containers:
        - name: gas-relayer
          image: yourregistry/gas-relayer:latest
          ports:
            - containerPort: 8080
          env:
            - name: RELAYER_PRIVATE_KEY
              valueFrom:
                secretKeyRef:
                  name: relayer-secrets
                  key: private-key
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: relayer-secrets
                  key: database-url
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 30
```

---

## Database Setup

```bash
# Create database
createdb gas_relayer

# Apply schema
psql gas_relayer < migrations/001_initial.sql

# Verify tables
psql gas_relayer -c "\dt"
```

The schema creates:
- `teams` — API key management
- `policies` — per-team relay rules
- `relay_jobs` — job lifecycle tracking
- `gas_usage` — quota enforcement
- `banned_users` — abuse prevention
- `relayer_wallets` — wallet/key metadata (no keys stored!)
- `relayer_nonces` — persistent nonce state

---

## Monitoring & Observability

### Logs

The relayer emits structured JSON logs with `tracing`. Key log events:

```
relay request received   { chain_id, from, to, gas }
signature verified       { signer }
job enqueued             { job_id }
relay transaction submitted { job_id, tx_hash, relayer, user, target, gas }
relay confirmed ✓         { job_id, tx_hash, block, gas_used }
job permanently failed   { reason, attempts }
```

Set `RUST_LOG` to control verbosity:
```bash
RUST_LOG=gas_relayer=debug,tower_http=trace,ethers=warn
```

### Prometheus Metrics

The server exposes metrics at `GET /metrics` (Prometheus format):

```
gas_relayer_requests_total{status="success"} 1234
gas_relayer_requests_total{status="failed"} 12
gas_relayer_queue_depth 5
gas_relayer_wallet_balance_wei{chain="1"} 1000000000000000000
gas_relayer_gas_price_gwei{chain="1"} 45
```

### Alerts to Set Up

| Alert | Threshold | Severity |
|---|---|---|
| Wallet balance low | < 0.1 ETH | Warning |
| Wallet balance critical | < 0.01 ETH | Critical |
| Job failure rate | > 5% over 5min | Warning |
| Queue depth | > 500 jobs | Warning |
| Gas price spike | > circuit breaker | Info |
| Stuck transactions | pending > 10min | Warning |

---

## Security Hardening

### Checklist

- [ ] Private key in KMS/Vault, not in env vars
- [ ] `allowed_targets` is an allowlist, not `Any`
- [ ] `allowed_selectors` restricts which functions can be called
- [ ] `deadline` enforced — reject requests older than 1 hour
- [ ] CORS restricted to your dApp's domain only
- [ ] TLS terminated at load balancer (HTTPS only)
- [ ] Rate limiting enabled per user address
- [ ] Daily gas quota per user set
- [ ] Circuit breaker gas price set per chain
- [ ] Database not exposed to the internet
- [ ] Relayer wallet is a dedicated address (not your personal wallet)
- [ ] Monitoring alerts configured on wallet balance

### What the Relayer Cannot Do (Even if Compromised)

- Cannot steal users' tokens (relayer never has custody)
- Cannot forge signatures (cryptographically impossible)
- Cannot replay transactions (nonces are on-chain)
- Cannot call contracts not in your allowlist (policy check)
- Cannot exceed your circuit breaker gas price

---

## Troubleshooting

### "nonce mismatch" errors

The user's nonce is stale. Common causes:
- Another relayer already processed a request for this user
- User submitted a direct transaction in between

Solution: Always fetch fresh nonce from `/nonce/:chain/:address` immediately before building the request. Never cache nonces on the client side.

### "signature mismatch" errors

The recovered signer doesn't match `request.from`. Common causes:
- Wrong `chainId` in the EIP-712 domain
- Wrong `verifyingContract` (forwarder address)
- Wrong domain `name` or `version`
- `data` field contains the wrong encoding

Solution: Fetch domain params from `/domain/:chainId` and use them exactly. Log the domain separator from the server startup and verify it matches what your frontend computes.

### Stuck transactions

The relayer submitted a transaction but it's stuck in the mempool. Common causes:
- Gas price too low (baseFee increased)
- Nonce gap (an earlier transaction got stuck)

Solution: The relayer auto-cancels stuck transactions after a timeout. You can also manually bump gas by sending a replacement transaction with a higher fee.

### "gas limit exceeded" policy errors

The user's `request.gas` exceeds `max_gas_per_request`. Common causes:
- Gas estimation on the client returned a high value
- Policy is set too low

Solution: Either increase `max_gas_per_request` in the policy, or clamp your client-side gas estimate.

### Database connection errors

```
Error: failed to connect to database
```

Check that:
- PostgreSQL is running
- `DATABASE_URL` is correct
- The `gas_relayer` database exists
- Migrations have been run

### "queue is full" errors

The in-memory queue is at capacity (default 1000). This means your workers can't process jobs fast enough.

Solutions:
- Increase `worker_count` in config
- Switch to Redis-backed queue for overflow
- Add more relayer instances (horizontal scaling)
