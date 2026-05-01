# Gas Relaying: ERC-712 & ERC-2771 In Depth

> How to build zero-fee UX for your users — without changing your smart contracts.

---

## Table of Contents

1. [The Problem: Gas UX is Broken](#the-problem)
2. [The Solution Architecture](#the-solution)
3. [ERC-712: Typed Structured Data Signing](#erc-712)
   - Domain Separators
   - Type Hashes
   - Encoding Pipeline
   - Security Properties
4. [ERC-2771: Meta-Transaction Standard](#erc-2771)
   - Trusted Forwarder Pattern
   - `_msgSender()` Override
   - Context Appending
5. [How They Work Together](#how-they-work-together)
   - End-to-End Flow
   - Security Model
6. [What Your Relayer Must Do](#what-your-relayer-must-do)
7. [Attack Vectors & Mitigations](#attack-vectors)
8. [Production Considerations](#production-considerations)

---

## 1. The Problem: Gas UX is Broken <a name="the-problem"></a>

Every Ethereum transaction must be paid for in ETH. This creates a massive onboarding barrier:

```
User wants to transfer USDC
  → User needs ETH for gas
    → User must buy ETH on an exchange
      → User must KYC on exchange
        → User gives up
```

For dApp teams targeting mainstream users (gaming, loyalty, ticketing, social), requiring users to hold ETH is a non-starter. **Gas relaying solves this** by letting a third party (the relayer) pay gas on behalf of users. The user signs a message, the relayer submits the transaction.

The core challenge: **how does the smart contract know who the actual user is**, if a relayer address is `msg.sender`?

---

## 2. The Solution Architecture <a name="the-solution"></a>

```
┌─────────────────────────────────────────────────────────────────────┐
│                        OVERVIEW                                      │
│                                                                      │
│  User (no ETH)          Relayer (has ETH)       Smart Contract      │
│      │                       │                        │             │
│      │  1. Sign typed msg    │                        │             │
│      │──────────────────────>│                        │             │
│      │                       │  2. Submit tx with     │             │
│      │                       │     user's sig appended│             │
│      │                       │────────────────────────>            │
│      │                       │                        │             │
│      │                       │         3. Contract reads real       │
│      │                       │            sender from calldata      │
│      │                       │            via ERC-2771              │
└─────────────────────────────────────────────────────────────────────┘
```

Two standards work in concert:
- **ERC-712** — defines *how the user signs* the meta-transaction (structured, human-readable, replay-protected)
- **ERC-2771** — defines *how the contract recovers* the original user from a relayed call

---

## 3. ERC-712: Typed Structured Data Signing <a name="erc-712"></a>

Before ERC-712, the only way to sign data was `eth_sign`, which hashes arbitrary bytes. This had two problems:

1. Wallets showed users an unreadable hex blob — phishing paradise
2. Signatures were replayable across contracts and chains

ERC-712 fixes both by defining a canonical encoding that wallets can *decode and display*.

### 3.1 The Domain Separator

Every ERC-712 message is namespaced to a specific **domain** — preventing cross-contract, cross-chain replay:

```solidity
struct EIP712Domain {
    string  name;              // Human-readable name, e.g. "MyForwarder"
    string  version;           // Contract version, e.g. "1"
    uint256 chainId;           // EIP-155 chain ID (prevents cross-chain replay)
    address verifyingContract; // Contract address (prevents cross-contract replay)
}
```

The domain separator is computed once and stored:

```solidity
bytes32 DOMAIN_SEPARATOR = keccak256(
    abi.encode(
        // Hash of the type string itself
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
        keccak256(bytes("MyForwarder")),   // keccak of string
        keccak256(bytes("1")),
        block.chainid,
        address(this)
    )
);
```

**Why hash strings?** `abi.encode` for dynamic types produces variable-length output. Hashing them produces a fixed 32-byte value that can be safely combined with `abi.encode`.

### 3.2 Type Hashes

Every message *type* also has a hash of its type string. For a meta-transaction:

```solidity
// The canonical type string — field order MATTERS, must match exactly
string constant FORWARD_REQUEST_TYPE =
    "ForwardRequest(address from,address to,uint256 value,uint256 gas,uint256 nonce,bytes data)";

bytes32 constant FORWARD_REQUEST_TYPEHASH = keccak256(bytes(FORWARD_REQUEST_TYPE));
```

This locks the schema. If an attacker changes the type (e.g., adds a field), the hash changes and verification fails.

### 3.3 The Full Encoding Pipeline

Given a `ForwardRequest` struct:

```
Final signed hash = keccak256(
    "\x19\x01"              ← EIP-191 prefix (prevents collision with eth_sign)
    ‖ DOMAIN_SEPARATOR      ← domain-specific salt
    ‖ structHash            ← hash of the actual message
)

structHash = keccak256(
    abi.encode(
        FORWARD_REQUEST_TYPEHASH,   ← locks the schema
        request.from,               ← address (32 bytes padded)
        request.to,                 ← address (32 bytes padded)
        request.value,              ← uint256
        request.gas,                ← uint256
        request.nonce,              ← uint256
        keccak256(request.data)     ← bytes are hashed before encoding
    )
)
```

The `\x19\x01` prefix is critical — it is specified by EIP-191 as the version byte for *structured data*. This makes it impossible for an EIP-712 hash to be a valid RLP-encoded transaction (which starts with different bytes), preventing transaction spoofing.

### 3.4 What the User Sees

Because the structure is fully typed, MetaMask and other wallets can decode and display:

```
Sign this request?

MyForwarder (Version 1)
Chain: Ethereum Mainnet

From:    0xUserAddress
To:      0xERC20Token
Value:   0 ETH
Action:  transfer(0xRecipient, 1000000)  ← wallets can decode data
Nonce:   42
```

### 3.5 Security Properties of ERC-712

| Property | How It's Achieved |
|---|---|
| No cross-chain replay | `chainId` in domain separator |
| No cross-contract replay | `verifyingContract` in domain separator |
| No signature reuse | `nonce` in message, tracked on-chain |
| Schema immutability | `typeHash` in structHash |
| No collision with transactions | `\x19\x01` EIP-191 prefix |
| Human-readable UX | Typed structure decoded by wallets |

---

## 4. ERC-2771: Meta-Transaction Standard <a name="erc-2771"></a>

ERC-2771 solves the identity problem: when a relayer calls a contract, `msg.sender` is the relayer, not the user. The standard defines a clean way for contracts to recover the *original* user.

### 4.1 The Trusted Forwarder

```solidity
contract MyToken is ERC2771Context {
    constructor(address trustedForwarder) 
        ERC2771Context(trustedForwarder) {}
    
    function transfer(address to, uint256 amount) external {
        // _msgSender() returns the REAL user, not the relayer
        _transfer(_msgSender(), to, amount);
    }
}
```

The contract trusts exactly one forwarder address. Only that forwarder can append a sender address to calldata.

### 4.2 Context Appending

The forwarder appends the original sender's address (20 bytes) to the end of the calldata when making the call:

```
Normal call calldata:
[ function selector (4) ][ arguments... ]

ERC-2771 relayed calldata:
[ function selector (4) ][ arguments... ][ original sender address (20) ]
                                          ↑ appended by forwarder
```

The contract's `_msgSender()` reads this suffix:

```solidity
function _msgSender() internal view override returns (address sender) {
    if (isTrustedForwarder(msg.sender) && msg.data.length >= 20) {
        // The last 20 bytes are the original sender
        assembly {
            sender := shr(96, calldataload(sub(calldatasize(), 20)))
        }
    } else {
        return super._msgSender(); // normal msg.sender
    }
}
```

### 4.3 The Forwarder's Responsibility

The **forwarder contract** is the trust anchor. It must:

1. Verify the ERC-712 signature matches the `from` field
2. Verify and increment the nonce to prevent replay
3. Execute the call with the sender address appended
4. Validate that sufficient gas was forwarded (EIP-2771 includes a gas stipend check)

```solidity
function execute(ForwardRequest calldata req, bytes calldata signature)
    external payable returns (bool, bytes memory)
{
    // 1. Verify signature
    require(verify(req, signature), "Invalid signature");
    
    // 2. Increment nonce atomically
    _nonces[req.from]++;
    
    // 3. Execute with sender appended
    (bool success, bytes memory result) = req.to.call{
        gas: req.gas,
        value: req.value
    }(abi.encodePacked(req.data, req.from));  // ← append sender
    
    // 4. Gas griefing check
    // Ensures the callee received at least req.gas
    assert(gasleft() > req.gas / 63);
    
    return (success, result);
}
```

### 4.4 Nonce Management

Nonces prevent replay attacks. The standard uses a simple incrementing nonce per sender address. Some implementations use bitmap nonces (ERC-4337 style) for unordered execution.

```
User nonce = 0 → signs request A (nonce=0) → relayed → nonce becomes 1
                → signs request B (nonce=1) → relayed → nonce becomes 2
                
Replay attempt: submit request A again → FAILS (nonce 0 already used)
```

---

## 5. How They Work Together <a name="how-they-work-together"></a>

### 5.1 End-to-End Flow

```
1. USER SIDE (off-chain)
   ─────────────────────
   User wants to call token.transfer(recipient, amount)
   
   Constructs ForwardRequest {
     from:  user_address,
     to:    token_contract,
     value: 0,
     gas:   100_000,
     nonce: await forwarder.getNonce(user_address),
     data:  token.interface.encodeFunctionData("transfer", [recipient, amount])
   }
   
   Signs: eth_signTypedData_v4(user_address, {
     domain: { name, version, chainId, verifyingContract: forwarder_address },
     types:  { ForwardRequest: [...fields] },
     message: request
   })
   
   Sends { request, signature } to relayer API


2. RELAYER SIDE (off-chain)
   ──────────────────────────
   Receives { request, signature }
   
   Validates off-chain:
   - Recovers signer from ERC-712 hash → must equal request.from
   - Checks nonce matches on-chain nonce
   - Checks request.to is in allowed contract list
   - Checks rate limits / quota for request.from
   - Estimates gas cost, checks budget
   
   If valid: submits forwarder.execute(request, signature) to blockchain
             pays gas itself


3. ON-CHAIN (forwarder.execute)
   ─────────────────────────────
   Verifies ERC-712 signature on-chain (second line of defense)
   Increments nonce[request.from]
   
   Calls:
     token_contract.call(
       abi.encodePacked(
         transfer.selector ++ abi.encode(recipient, amount),
         user_address  ← appended
       )
     )


4. ON-CHAIN (token contract, ERC2771Context)
   ────────────────────────────────────────────
   msg.sender = forwarder_address (trusted)
   _msgSender() reads last 20 bytes of calldata = user_address
   
   _transfer(user_address, recipient, amount)  ✓
```

### 5.2 Security Model

```
Trust chain:

User ──signs──> ForwardRequest
                    │
                    ▼
              ERC-712 signature  (cryptographic, can't be forged)
                    │
                    ▼
              Forwarder.execute  (on-chain, verifies sig + nonce)
                    │
                    ▼
              token._msgSender() (reads user from trusted calldata suffix)
                    │
                    ▼
              Correct user identity ✓
```

The contract's trust is entirely in the **forwarder contract address**, not in the relayer server. A compromised relayer can:
- Refuse to relay (liveness failure, not security failure)
- Relay replayed transactions → FAILS (nonce check)
- Relay modified requests → FAILS (ERC-712 signature check)
- Impersonate users → FAILS (can't forge signatures)

The only attack vector is if the forwarder contract itself is malicious or buggy.

---

## 6. What Your Relayer Must Do <a name="what-your-relayer-must-do"></a>

A production relayer is more than a signature submitter. Here is the complete responsibility list:

### 6.1 Off-Chain Signature Verification

Never submit to chain without first verifying off-chain. This saves gas on failures.

```rust
// Pseudocode
fn verify_request(req: &ForwardRequest, sig: &[u8]) -> Result<()> {
    let hash = erc712_hash(req, &domain_separator);
    let recovered = ecrecover(hash, sig)?;
    ensure!(recovered == req.from, "signer mismatch");
    
    let on_chain_nonce = forwarder.get_nonce(req.from).await?;
    ensure!(req.nonce == on_chain_nonce, "nonce mismatch");
    Ok(())
}
```

### 6.2 Gas Estimation & Limits

```rust
// Must account for:
// 1. Base transaction cost (~21,000)
// 2. Forwarder overhead (~40,000)
// 3. req.gas (user-specified inner call gas)
// 4. Calldata cost (4 gas/zero byte, 16 gas/non-zero byte)

let estimated = provider.estimate_gas(tx).await?;
let with_buffer = estimated * 12 / 10; // 20% buffer
```

### 6.3 Nonce Management for Relayer's Own Wallet

The relayer submits many transactions. It must manage its *own* nonces carefully to avoid stuck transactions:

- Use a nonce manager with atomic increment
- Track pending transactions per nonce
- Implement replacement (EIP-1559 bump) for stuck txs
- Support multiple concurrent signer addresses

### 6.4 Authorization / Policy

```rust
// Each relayed request must pass your policy:
// - Is the target contract on the allowlist?
// - Does this user have remaining quota?
// - Is the gas limit within bounds?
// - Has the request expired (deadline field)?
// - Is the request.from address banned?
```

### 6.5 MEV / Front-Running Protection

Meta-transactions in the mempool can be front-run. Mitigations:
- Private mempool submission (Flashbots Protect, bloXroute)
- Commit-reveal schemes
- Fast confirmation (high priority fee)

---

## 7. Attack Vectors & Mitigations <a name="attack-vectors"></a>

| Attack | Description | Mitigation |
|---|---|---|
| **Replay** | Submit same signed request twice | On-chain nonce, off-chain nonce check |
| **Cross-chain replay** | Submit Mainnet sig on testnet | `chainId` in domain separator |
| **Front-running** | Snipe tx from mempool, reorder | Private RPC, high priority fees |
| **Gas griefing** | User specifies too-low `gas` causing contract revert but relayer still pays | Simulate before submit; charge user for failures |
| **Allowance drain** | User pre-approves, then relayer keeps submitting | Rate limits, quota, deadline in request |
| **Forwarder substitution** | Malicious forwarder doesn't append sender correctly | Use audited OpenZeppelin MinimalForwarder |
| **Domain mismatch** | Signature for wrong contract | Verify domain separator matches expected values |
| **Sig malleability** | s-value flipping on secp256k1 | Use `ecrecover` with s ≤ secp256k1n/2 check |
| **Deadline bypass** | Old signed requests replayed | Include `deadline` field; check on-chain |

---

## 8. Production Considerations <a name="production-considerations"></a>

### Gas Price Strategy
- Use EIP-1559 (baseFee + priorityFee)
- Monitor baseFee via `eth_feeHistory`
- Set `maxFeePerGas` = 2× current baseFee for safety
- Alert when gas prices spike above budget threshold

### Multi-Chain Support
- Maintain separate domain separators per chain
- Separate signer wallets per chain (key isolation)
- Chain-specific nonce managers
- Chain-specific contract addresses

### Relayer Wallet Security
- Hardware wallet / HSM for production signers
- Never store private keys in environment variables in prod
- Rotate keys on compromise
- Multi-sig for treasury management

### Monitoring
- Track: relay success rate, gas spend per request, nonce gaps, queue depth
- Alert on: wallet balance low, stuck transactions, error rate spikes

### Persistent Queue
- Use Redis or PostgreSQL for relay queue
- Implement dead-letter queue for failed relays
- Retry with exponential backoff
- Idempotency keys to prevent double-submission

---

*This document accompanies the Rust gas relayer implementation. See `src/` for the full production server.*
