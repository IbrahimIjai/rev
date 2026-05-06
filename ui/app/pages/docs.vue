<script setup lang="ts">
const toast = useToast()

async function copy(text: string) {
  await navigator.clipboard.writeText(text)
  toast.add({ title: 'Copied', color: 'success' })
}

const FORWARDER = '0x3BaF50F4152Bb2C3F0E27693600c6C6c56D9D0E7'
const CHAIN_ID = 84532

const contractCode = `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import "@openzeppelin/contracts/metatx/ERC2771Context.sol";

contract GaslessNFT is ERC721URIStorage, ERC2771Context {
    uint256 private _nextTokenId;

    constructor(address trustedForwarder)
        ERC721("MyGaslessNFT", "GNFT")
        ERC2771Context(trustedForwarder)
    {}

    /// @dev Anyone can mint — gas is paid by the relayer, not the user.
    function mint(string memory tokenURI) external returns (uint256) {
        uint256 tokenId = _nextTokenId++;
        _mint(_msgSender(), tokenId);
        _setTokenURI(tokenId, tokenURI);
        return tokenId;
    }

    // Required overrides so ERC2771Context._msgSender() wins over Context
    function _msgSender() internal view override(Context, ERC2771Context) returns (address) {
        return ERC2771Context._msgSender();
    }
    function _msgData() internal view override(Context, ERC2771Context) returns (bytes calldata) {
        return ERC2771Context._msgData();
    }
    function _contextSuffixLength() internal view override(Context, ERC2771Context) returns (uint256) {
        return ERC2771Context._contextSuffixLength();
    }
}`

const deployCode = `# 1. Install deps
forge install OpenZeppelin/openzeppelin-contracts

# 2. Add remapping to foundry.toml
# remappings = ["@openzeppelin/contracts/=lib/openzeppelin-contracts/contracts/"]

# 3. Deploy — pass the forwarder address as constructor arg
forge create src/GaslessNFT.sol:GaslessNFT \\
  --constructor-args ${FORWARDER} \\
  --rpc-url https://sepolia.base.org \\
  --private-key $PRIVATE_KEY`

const clientCode = `import { createWalletClient, createPublicClient, http, encodeFunctionData } from 'viem'
import { baseSepolia } from 'viem/chains'

const FORWARDER   = '${FORWARDER}'
const NFT_ADDRESS = '0xYOUR_DEPLOYED_NFT'
const API_KEY     = 'your-api-key'
const TOKEN_URI   = 'ipfs://your-metadata-cid'

const domain = {
  name: 'GasRelayForwarder',
  version: '1',
  chainId: ${CHAIN_ID},
  verifyingContract: FORWARDER,
}

const types = {
  ForwardRequest: [
    { name: 'from',     type: 'address' },
    { name: 'to',       type: 'address' },
    { name: 'value',    type: 'uint256' },
    { name: 'gas',      type: 'uint256' },
    { name: 'nonce',    type: 'uint256' },
    { name: 'deadline', type: 'uint48'  },
    { name: 'data',     type: 'bytes'   },
  ],
}

// 1. Read the user's current nonce from the forwarder contract
const nonce = await publicClient.readContract({
  address: FORWARDER,
  abi: [{ name: 'nonces', type: 'function', inputs: [{ type: 'address' }], outputs: [{ type: 'uint256' }] }],
  functionName: 'nonces',
  args: [userAddress],
})

// 2. Build the forward request
const request = {
  from:     userAddress,
  to:       NFT_ADDRESS,
  value:    0n,
  gas:      200000n,
  nonce,
  deadline: BigInt(Math.floor(Date.now() / 1000) + 3600), // 1 hour
  data:     encodeFunctionData({
    abi: [{ name: 'mint', type: 'function', inputs: [{ name: 'tokenURI', type: 'string' }] }],
    functionName: 'mint',
    args: [TOKEN_URI],
  }),
}

// 3. Sign with EIP-712 — user signs, no ETH needed
const signature = await walletClient.signTypedData({ domain, types, primaryType: 'ForwardRequest', message: request })

// 4. Send to the relayer — relayer pays gas
const res = await fetch('http://localhost:8080/relay', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json', 'X-API-Key': API_KEY },
  body: JSON.stringify({ request, signature }),
})

const { jobId } = await res.json()

// 5. Poll for confirmation
const status = await fetch(\`http://localhost:8080/relay/\${jobId}\`).then(r => r.json())
console.log(status) // { status: 'confirmed', txHash: '0x...' }`
</script>

<template>
  <div class="max-w-3xl mx-auto space-y-10 pb-20">

    <!-- Header -->
    <div class="space-y-2 pt-2">
      <div class="flex items-center gap-2">
        <div class="w-7 h-7 rounded-lg bg-primary/10 flex items-center justify-center">
          <UIcon name="i-heroicons-book-open" class="w-4 h-4 text-primary" />
        </div>
        <h1 class="text-2xl font-black text-highlighted tracking-tight">Deploy a Gasless NFT</h1>
      </div>
      <p class="text-sm text-muted font-medium">
        Ship an ERC-721 where users mint for free — your gas tank covers the fees.
        Uses OpenZeppelin's <code class="text-primary text-xs">ERC2771Context</code> + our shared forwarder on Base Sepolia.
      </p>
    </div>

    <!-- How it works -->
    <section class="space-y-4">
      <h2 class="text-sm font-black text-highlighted uppercase tracking-widest">How it works</h2>
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <div v-for="(step, i) in [
          { icon: 'i-heroicons-pencil-square', label: 'User signs', desc: 'User signs an EIP-712 ForwardRequest off-chain. No ETH needed.' },
          { icon: 'i-heroicons-paper-airplane', label: 'Relayer submits', desc: 'Your API call forwards the signed request. Relayer pays gas from your tank.' },
          { icon: 'i-heroicons-check-badge', label: 'NFT minted', desc: 'Forwarder verifies the signature and calls mint(). _msgSender() returns the real user.' },
        ]" :key="i" class="rounded-xl border border-default bg-default p-4 space-y-2">
          <div class="flex items-center gap-2">
            <span class="text-xs font-black text-dimmed">{{ i + 1 }}</span>
            <UIcon :name="step.icon" class="w-4 h-4 text-primary" />
            <span class="text-xs font-bold text-toned">{{ step.label }}</span>
          </div>
          <p class="text-xs text-dimmed leading-relaxed">{{ step.desc }}</p>
        </div>
      </div>
    </section>

    <!-- Forwarder contract -->
    <section class="space-y-3">
      <h2 class="text-sm font-black text-highlighted uppercase tracking-widest">Deployed Forwarder</h2>
      <p class="text-xs text-muted">
        This is the shared <code class="text-primary">ERC2771Forwarder</code> on Base Sepolia. Pass this address to your contract constructor — do not deploy your own.
      </p>
      <div class="rounded-xl border border-default bg-default overflow-hidden">
        <div class="px-4 py-2.5 bg-muted border-b border-default flex items-center justify-between">
          <span class="text-xs font-semibold text-dimmed">Base Sepolia · Chain {{ CHAIN_ID }}</span>
          <a
            :href="`https://sepolia.basescan.org/address/${FORWARDER}`"
            target="_blank"
            class="text-xs text-primary hover:underline font-semibold flex items-center gap-1"
          >
            View on Basescan
            <UIcon name="i-heroicons-arrow-top-right-on-square" class="w-3 h-3" />
          </a>
        </div>
        <div class="flex items-center gap-2 px-4 py-3">
          <code class="flex-1 text-xs font-mono text-toned break-all">{{ FORWARDER }}</code>
          <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copy(FORWARDER)" />
        </div>
        <div class="px-4 pb-3 flex flex-wrap gap-3 text-xs text-dimmed">
          <span><span class="text-toned font-semibold">Name:</span> GasRelayForwarder</span>
          <span><span class="text-toned font-semibold">Version:</span> 1</span>
          <span><span class="text-toned font-semibold">Standard:</span> ERC-2771</span>
        </div>
      </div>
    </section>

    <!-- Step 1: Write contract -->
    <section class="space-y-3">
      <div class="flex items-center gap-2">
        <span class="w-5 h-5 rounded-full bg-primary text-inverted text-xs font-black flex items-center justify-center shrink-0">1</span>
        <h2 class="text-sm font-black text-highlighted uppercase tracking-widest">Write your contract</h2>
      </div>
      <p class="text-xs text-muted leading-relaxed">
        Inherit from both <code class="text-primary">ERC721URIStorage</code> and <code class="text-primary">ERC2771Context</code>.
        The three overrides at the bottom are required — they tell Solidity to use ERC2771's <code class="text-primary">_msgSender()</code>
        instead of the base <code class="text-primary">Context</code> one. That's the whole trick: <code class="text-primary">_msgSender()</code>
        returns the original user, not the relayer wallet.
      </p>
      <div class="rounded-xl border border-default bg-default overflow-hidden">
        <div class="px-4 py-2.5 bg-muted border-b border-default flex items-center justify-between">
          <span class="text-xs font-semibold text-dimmed">src/GaslessNFT.sol</span>
          <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" label="Copy" @click="copy(contractCode)" />
        </div>
        <pre class="p-4 text-xs font-mono text-toned overflow-x-auto leading-relaxed whitespace-pre">{{ contractCode }}</pre>
      </div>
    </section>

    <!-- Step 2: Deploy -->
    <section class="space-y-3">
      <div class="flex items-center gap-2">
        <span class="w-5 h-5 rounded-full bg-primary text-inverted text-xs font-black flex items-center justify-center shrink-0">2</span>
        <h2 class="text-sm font-black text-highlighted uppercase tracking-widest">Deploy with Foundry</h2>
      </div>
      <p class="text-xs text-muted leading-relaxed">
        Pass the forwarder address as the only constructor argument. Grab some Base Sepolia ETH from
        <a href="https://faucet.quicknode.com/base/sepolia" target="_blank" class="text-primary hover:underline">QuickNode faucet</a> if your wallet is empty.
      </p>
      <div class="rounded-xl border border-default bg-default overflow-hidden">
        <div class="px-4 py-2.5 bg-muted border-b border-default flex items-center justify-between">
          <span class="text-xs font-semibold text-dimmed">Terminal</span>
          <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" label="Copy" @click="copy(deployCode)" />
        </div>
        <pre class="p-4 text-xs font-mono text-toned overflow-x-auto leading-relaxed whitespace-pre">{{ deployCode }}</pre>
      </div>
      <div class="rounded-xl border border-warning/20 bg-warning/5 px-4 py-3 flex gap-2">
        <UIcon name="i-heroicons-light-bulb" class="w-4 h-4 text-warning shrink-0 mt-0.5" />
        <p class="text-xs text-muted leading-relaxed">
          Save the deployed contract address from the output — you'll need it in the next step.
        </p>
      </div>
    </section>

    <!-- Step 3: Create project -->
    <section class="space-y-3">
      <div class="flex items-center gap-2">
        <span class="w-5 h-5 rounded-full bg-primary text-inverted text-xs font-black flex items-center justify-center shrink-0">3</span>
        <h2 class="text-sm font-black text-highlighted uppercase tracking-widest">Create a project &amp; fund the gas tank</h2>
      </div>
      <div class="rounded-xl border border-default bg-default p-4 space-y-3 text-xs text-muted leading-relaxed">
        <div class="flex gap-2">
          <UIcon name="i-heroicons-check-circle" class="w-4 h-4 text-success shrink-0 mt-0.5" />
          <span>Go to <NuxtLink to="/projects" class="text-primary hover:underline font-semibold">Projects</NuxtLink> → <strong class="text-toned">New Project</strong>. Pick Base Sepolia and paste the forwarder address above.</span>
        </div>
        <div class="flex gap-2">
          <UIcon name="i-heroicons-check-circle" class="w-4 h-4 text-success shrink-0 mt-0.5" />
          <span>Copy the <strong class="text-toned">API key</strong> shown after creation — it won't appear again.</span>
        </div>
        <div class="flex gap-2">
          <UIcon name="i-heroicons-check-circle" class="w-4 h-4 text-success shrink-0 mt-0.5" />
          <span>Send some Base Sepolia ETH to the <strong class="text-toned">Gas Tank</strong> address shown on the project page. The relayer draws from it to pay gas for every mint.</span>
        </div>
      </div>
    </section>

    <!-- Step 4: Integrate -->
    <section class="space-y-3">
      <div class="flex items-center gap-2">
        <span class="w-5 h-5 rounded-full bg-primary text-inverted text-xs font-black flex items-center justify-center shrink-0">4</span>
        <h2 class="text-sm font-black text-highlighted uppercase tracking-widest">Send a gasless mint from your app</h2>
      </div>
      <p class="text-xs text-muted leading-relaxed">
        Use <code class="text-primary">viem</code> (or ethers) to build and sign the <code class="text-primary">ForwardRequest</code>,
        then POST it to the relay endpoint. The user signs — never spends ETH.
      </p>
      <div class="rounded-xl border border-default bg-default overflow-hidden">
        <div class="px-4 py-2.5 bg-muted border-b border-default flex items-center justify-between">
          <span class="text-xs font-semibold text-dimmed">mint.ts · viem</span>
          <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" label="Copy" @click="copy(clientCode)" />
        </div>
        <pre class="p-4 text-xs font-mono text-toned overflow-x-auto leading-relaxed whitespace-pre">{{ clientCode }}</pre>
      </div>
    </section>

    <!-- Relay request reference -->
    <section class="space-y-3">
      <h2 class="text-sm font-black text-highlighted uppercase tracking-widest">Relay Request Reference</h2>
      <div class="rounded-xl border border-default bg-default overflow-hidden divide-y divide-default text-xs">
        <div class="grid grid-cols-3 px-4 py-2 bg-muted text-dimmed font-semibold uppercase tracking-wide">
          <span>Field</span><span>Type</span><span>Description</span>
        </div>
        <div v-for="row in [
          ['from', 'address', 'User\'s wallet — who the NFT is minted to'],
          ['to', 'address', 'Your deployed NFT contract address'],
          ['value', 'uint256', 'Always 0 for non-payable calls'],
          ['gas', 'uint256', 'Gas limit — 200 000 is safe for a simple mint'],
          ['nonce', 'uint256', 'Fetched from forwarder.nonces(userAddress)'],
          ['deadline', 'uint48', 'Unix timestamp — signature expires after this'],
          ['data', 'bytes', 'ABI-encoded mint(tokenURI) calldata'],
          ['signature', 'bytes', 'EIP-712 sig over the fields above'],
        ]" :key="row[0]" class="grid grid-cols-3 px-4 py-2.5 text-muted">
          <code class="text-primary font-mono">{{ row[0] }}</code>
          <span class="text-dimmed font-mono">{{ row[1] }}</span>
          <span>{{ row[2] }}</span>
        </div>
      </div>
    </section>

  </div>
</template>
