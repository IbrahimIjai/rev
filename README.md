# ⛽ Gas Relayer

> Production-grade ERC-2771 meta-transaction relayer built in Rust.  
> Let your dApp users transact on Ethereum without ever holding ETH.

---

## What It Does

This relayer allows dApp teams to offer **gasless transactions** to their users. The user signs a message with their wallet (no ETH needed), your relayer pays the gas, and the smart contract sees the correct user as the sender.

Built on two Ethereum standards:
- **ERC-712** — typed structured data signing (secure, human-readable)
- **ERC-2771** — trusted forwarder pattern (contract identity recovery)

## Architecture

```
User (no ETH) ──signs EIP-712──► Relayer API ──submits tx──► MinimalForwarder ──appends sender──► Your Contract
                                       │                            │                                    │
                                   validates                   verifies sig                      _msgSender()
                                   policy, nonce              increments nonce                  = real user ✓
```

## Quick Start

```bash
cp .env.example .env
# Edit .env with your values

cargo build --release
./target/release/gas-relayer
```

## Documentation

- **[ERC-712 & ERC-2771 Deep Dive](docs/ERC712_ERC2771_DEEP_DIVE.md)** — how the standards work and why
- **[Usage Guide](docs/USAGE.md)** — installation, CLI, API reference, integration guide
- **[Architecture Diagram](docs/ARCHITECTURE.md)** — full system diagram

## Project Structure

```
gas-relayer/
├── src/
│   ├── main.rs                   # Entry point, wiring
│   ├── config/mod.rs             # Config loading (env + yaml)
│   ├── crypto/
│   │   ├── eip712.rs             # EIP-712 hashing + signature verification
│   │   └── key_manager.rs        # Multi-provider key management (local/KMS/Vault)
│   ├── models/mod.rs             # ForwardRequest, RelayJob, domain types
│   ├── handlers/relay.rs         # HTTP handlers (axum)
│   └── services/
│       ├── policy.rs             # Policy engine — what gets relayed
│       ├── nonce_manager.rs      # User + relayer nonce management
│       ├── relay_executor.rs     # Transaction building, signing, submission
│       └── queue.rs              # Job queue + worker pool
├── contracts/
│   └── MinimalForwarder.sol      # ERC-2771 forwarder + ERC2771Context
├── migrations/
│   └── 001_initial.sql           # Full PostgreSQL schema
├── docs/
│   ├── ERC712_ERC2771_DEEP_DIVE.md
│   └── USAGE.md
├── config.yaml                   # Configuration template
├── .env.example                  # Environment variable template
├── docker-compose.yml            # Full stack (relayer + postgres + redis + anvil)
└── Dockerfile                    # Production image
```

## Key Security Properties

| Threat | Defense |
|---|---|
| Replay attacks | On-chain nonce on forwarder contract |
| Cross-chain replay | `chainId` in EIP-712 domain separator |
| Forged signatures | secp256k1 ECDSA verification |
| Key exposure | AWS KMS / HashiCorp Vault — key never in memory |
| Abuse / overspend | Policy engine: quotas, rate limits, contract allowlist |
| Gas price spikes | Per-chain circuit breaker threshold |

## License

MIT
