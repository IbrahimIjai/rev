# revv

A production-grade ERC-2771 meta-transaction relayer — let your users transact on-chain without ever holding ETH.

---

## About

revv is a full-stack gasless transaction infrastructure. Teams deposit ETH into a **project gas tank**, and their users can interact with any ERC-2771-compatible smart contract completely for free — no wallet, no ETH, no friction.

The system handles signature verification, nonce management, transaction construction, gas estimation, on-chain submission, and retry logic. Projects manage their gas budgets through a dashboard UI.

---

## Motivation

I built this to deeply understand two Ethereum standards I kept reading about but never fully grasped by just reading specs:

**EIP-712** — How do you sign structured data in a way that's human-readable in MetaMask, replay-safe across chains, and tamper-evident? Turns out there's a precise encoding: type hash + domain separator + message hash → `eth_sign(keccak256("\x19\x01" || domainHash || msgHash))`.

**ERC-2771** — How does a smart contract know who the *real* user is when a relayer submits the transaction? The forwarder appends the original sender's address as the last 20 bytes of calldata, and the contract recovers it via `_msgSender()` from `ERC2771Context`.

Reading about these things is one thing. Implementing them from scratch — the encoding, the hashing, the sig recovery, the nonce tracking, the actual raw transaction signing — is how you actually learn them.

---

## Demo

<video src="https://raw.githubusercontent.com/IbrahimIjai/rev/main/ui/public/demo-fast.mp4" controls width="100%" height="400"></video>

---

## Project Structure

```
zero/gas-relayer/
├── gas-relayer/              # Rust backend (Axum + SeaORM)
│   └── src/
│       ├── main.rs           # Server entry point, router wiring
│       ├── handlers/
│       │   ├── relay.rs      # POST /relay, GET /relay/:id, /nonce, /domain, /health
│       │   ├── projects.rs   # Project CRUD, API key management, gas tank limits
│       │   └── auth.rs       # SIWE (Sign-In with Ethereum) — nonce + verify
│       ├── services/
│       │   ├── relay_executor.rs  # Builds + signs + submits raw EIP-1559 transactions
│       │   ├── nonce_manager.rs   # Per-user nonce cache + relayer nonce tracking
│       │   ├── policy.rs          # Gas quotas, rate limits, allowed targets
│       │   └── queue.rs           # In-memory job queue + worker pool with retries
│       ├── crypto/
│       │   └── eip712.rs     # EIP-712 domain separator + ForwardRequest hashing
│       ├── db/
│       │   └── entities/     # SeaORM entities (project, gas_tank, relay_job, api_key, …)
│       └── migration/        # Sea-ORM migrations (schema auto-applied at startup)
│
├── contracts/                # Foundry project
│   └── src/
│       ├── GaslessNFT.sol    # ERC721 + ERC2771Context (gasless minting demo)
│       └── GaslessToken.sol  # ERC20 + ERC2771Context (gasless transfer demo)
│
├── ui/                       # Nuxt 4 dashboard (SSR disabled, bun)
│   └── app/
│       ├── pages/
│       │   ├── projects/     # Project list + detail pages
│       │   └── docs.vue      # Embedded API docs
│       ├── stores/           # Pinia stores (auth, projects)
│       └── composables/      # useRelayer, useWallet, …
│
├── erc721-minter/            # Rust CLI script — demos a full gasless NFT mint
│   └── src/main.rs
│
├── docker-compose.yml        # Spins up everything: relayer + ui + postgres + redis
└── docs/                     # Additional documentation
```

---

## Tech Stack

| Layer | Technology |
|---|---|
| **Backend** | Rust, Axum (HTTP), Tokio (async runtime) |
| **Database** | PostgreSQL 16 via SeaORM + Sea-ORM migrations |
| **Cache / Queue** | Redis 7 (rate limiting, session nonces) |
| **Ethereum** | Alloy (provider, signing, ABI encoding), Foundry (contracts) |
| **Frontend** | Nuxt 4, Vue 3, Pinia, bun |
| **Crypto** | AES-256-GCM (private key encryption at rest), secp256k1 ECDSA |
| **Auth** | SIWE (Sign-In with Ethereum) + JWT |

---

## How the Relayer Works

```bash
docker compose up --build
# UI → http://localhost:3000  |  Relayer API → http://localhost:8080
```


### The Gasless Flow

A user wants to mint an NFT but has no ETH. Here's what happens end-to-end:

```
1. dApp fetches nonce
   GET /nonce/84532/0xUSER
   ← { nonce: "5" }

2. dApp builds a ForwardRequest and signs it (EIP-712)
   ForwardRequest {
     from:     0xUSER,
     to:       0xNFT_CONTRACT,
     value:    0,
     gas:      200000,
     nonce:    5,
     deadline: now + 5min,
     data:     mint() calldata
   }
   → user signs this with their private key (no ETH spent)

3. dApp submits to relayer
   POST /relay
   { request: ForwardRequest, signature: "0x...", apiKey: "pk_live_..." }

4. Relayer validates
   ✓ API key → look up project
   ✓ EIP-712 signature → recover signer == request.from
   ✓ Policy check → within daily gas quota, rate limit OK
   ✓ Enqueue job → respond with { jobId }

5. Worker picks up the job
   → Load project's gas tank from DB
   → Decrypt private key (AES-256-GCM)
   → Fetch gas tank nonce from chain
   → Estimate gas for the forwarder.execute() call
   → Build EIP-1559 transaction
   → Sign with gas tank key
   → send_raw_transaction (the gas tank pays!)

6. Forwarder contract (on-chain)
   function execute(ForwardRequest req, bytes sig) external {
       require(verify(req, sig));          // checks EIP-712 sig
       req.from.nonce++;                   // replay protection
       req.to.call{ gas: req.gas }(        // call target
           abi.encodePacked(req.data, req.from)  // append real sender
       );
   }

7. NFT contract receives the call
   function _msgSender() returns (address) {
       // last 20 bytes of calldata = the real user (appended by forwarder)
       return ERC2771Context._msgSender();
   }
   // mint() uses _msgSender() → NFT goes to the user, not the relayer!
```

### Job Flow Diagram

```
POST /relay
     │
     ▼
┌─────────────┐
│  Relay      │  validate API key
│  Handler    │  verify EIP-712 sig
│             │  check policy (quota, rate)
└──────┬──────┘
       │ enqueue
       ▼
┌─────────────┐
│  Job Queue  │  in-memory mpsc channel (capacity: 1000)
│  (Redis)    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Worker     │  4 concurrent workers
│  Pool       │
└──────┬──────┘
       │
       ├── load gas tank (DB)
       ├── decrypt private key (AES-GCM)
       ├── fetch relayer nonce (RPC)
       ├── estimate gas (eth_estimateGas)
       ├── build TxEip1559
       ├── sign (secp256k1)
       └── send_raw_transaction
             │
             ▼
     ┌───────────────┐
     │  Base Sepolia │  ERC2771Forwarder.execute()
     │  (or mainnet) │       ↓
     └───────────────┘  Your Contract._msgSender() = real user
```

### Retry Logic

Failed jobs are retried up to 3 times with exponential backoff (5s → 10s → 20s). Reverted transactions are marked as permanent failures (quota refunded). Network errors and timeouts are retried.

### Per-Project Gas Tanks

Each project has its own Ethereum wallet (gas tank) stored encrypted in PostgreSQL. When a job runs, the relayer decrypts that project's key, uses it to sign and pay for the transaction, and updates usage records. Projects never share gas budgets — one project's spending can't affect another.

---

## Contracts (Base Sepolia)

| Contract | Address |
|---|---|
| ERC2771Forwarder | `0xDb78D27B530ed9c0fC582f53b558b75c5ab63A90` |
| GaslessNFT | `0xBe9ec79854e459F38E0B868A0c3429AAbf6784b2` |
| GaslessToken | `0x1399d6e24F5299beC40A8c1362b2167982265A79` |

The `GaslessNFT` and `GaslessToken` contracts inherit from both `ERC721`/`ERC20` and `ERC2771Context`. They override `_msgSender()` and `_contextSuffixLength()` to correctly recover the original user from the forwarder's appended calldata.

---

## Running Locally with Docker

The entire stack — Rust relayer, Nuxt UI, PostgreSQL, Redis — starts with one command:

```bash
# 1. Set required env vars
export FORWARDER_ADDRESS=0xDb78D27B530ed9c0fC582f53b558b75c5ab63A90
export RPC_URL=https://sepolia.base.org
export CHAIN_ID=84532

# 2. (Optional) customize secrets
export JWT_SECRET=your-secret-here
export ENCRYPTION_SECRET=your-32-byte-secret-here!!!!

# 3. Boot everything
docker compose up --build

# Services:
#   UI        → http://localhost:3000
#   Relayer   → http://localhost:8080
#   Postgres  → localhost:5432
#   Redis     → localhost:6379
```

Migrations run automatically when the relayer starts. No manual DB setup needed.

### Testing a Gasless Mint

After the stack is running:

1. Open `http://localhost:3000`
2. Connect your wallet (Base Sepolia)
3. Create a project and fund its gas tank with Base Sepolia ETH
4. Run the Rust minter script:

```bash
cd erc721-minter
# edit src/main.rs: paste your API key and private key
cargo run
# prints: https://base-sepolia.blockscout.com/tx/0x...
```

---

## API Reference

| Method | Path | Description |
|---|---|---|
| `POST` | `/relay` | Submit a meta-transaction |
| `GET` | `/relay/:id` | Check job status |
| `GET` | `/nonce/:chainId/:address` | Get user's current forwarder nonce |
| `GET` | `/domain/:chainId` | Get EIP-712 domain info |
| `GET` | `/health` | Health check |
| `GET` | `/api/auth/nonce` | SIWE nonce |
| `POST` | `/api/auth/verify` | SIWE verify + JWT |
| `POST` | `/api/projects` | Create project |
| `GET` | `/api/projects` | List projects |
| `GET` | `/api/projects/:id` | Get project |
| `PUT` | `/api/projects/:id/limits` | Update gas limits |
| `POST` | `/api/projects/:id/api-keys` | Create API key |

---

## License

MIT
