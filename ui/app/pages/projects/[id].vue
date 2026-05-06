<script setup lang="ts">
import { useBalance } from '@wagmi/vue'
import { formatEther } from 'viem'
import { CONFIG } from '~/lib/config'

const route = useRoute()
const api = useApi()
const toast = useToast()

const projectId = route.params.id as string

const { data: project, pending, refresh } = useAsyncData(
  `project-${projectId}`,
  () => api.projects.get(projectId),
)

// ─── Spending limits form ─────────────────────────────────────────────────────
const limitsForm = reactive({
  maxGasPerRequest: '',
  maxGasPriceGwei: 150,
  rateLimitPerMinute: 10,
  dailyGasQuotaPerUser: '',
  webhookUrl: '',
})
const savingLimits = ref(false)

watch(project, (p) => {
  if (!p?.spendingLimits) return
  limitsForm.maxGasPerRequest = p.spendingLimits.maxGasPerRequest ?? ''
  limitsForm.maxGasPriceGwei = p.spendingLimits.maxGasPriceGwei ?? 150
  limitsForm.rateLimitPerMinute = p.spendingLimits.rateLimitPerMinute ?? 10
  limitsForm.dailyGasQuotaPerUser = p.spendingLimits.dailyGasQuotaPerUser ?? ''
  limitsForm.webhookUrl = p.spendingLimits.webhookUrl ?? ''
}, { immediate: true })

async function saveLimits() {
  savingLimits.value = true
  try {
    await api.projects.updateLimits(projectId, {
      maxGasPerRequest: limitsForm.maxGasPerRequest || undefined,
      maxGasPriceGwei: limitsForm.maxGasPriceGwei,
      rateLimitPerMinute: limitsForm.rateLimitPerMinute,
      dailyGasQuotaPerUser: limitsForm.dailyGasQuotaPerUser || undefined,
      webhookUrl: limitsForm.webhookUrl || undefined,
    } as Parameters<typeof api.projects.updateLimits>[1])
    toast.add({ title: 'Limits saved', color: 'success' })
    await refresh()
  }
  catch (err: unknown) {
    const msg = (err as { data?: { error?: string } })?.data?.error ?? 'Failed to save'
    toast.add({ title: 'Error', description: msg, color: 'error' })
  }
  finally {
    savingLimits.value = false
  }
}

// ─── New API key ──────────────────────────────────────────────────────────────
const showNewKey = ref(false)
const newKeyName = ref('')
const creatingKey = ref(false)
const newKeyResult = ref<{ apiKey: string; name: string } | null>(null)
const showKeyResult = ref(false)

async function createKey() {
  if (!newKeyName.value.trim()) return
  creatingKey.value = true
  try {
    const result = await api.projects.createApiKey(projectId, newKeyName.value.trim())
    newKeyResult.value = result
    newKeyName.value = ''
    showNewKey.value = false
    showKeyResult.value = true
  }
  catch (err: unknown) {
    const msg = (err as { data?: { error?: string } })?.data?.error ?? 'Failed to create key'
    toast.add({ title: 'Error', description: msg, color: 'error' })
  }
  finally {
    creatingKey.value = false
  }
}

async function copyText(text: string) {
  await navigator.clipboard.writeText(text)
  toast.add({ title: 'Copied', color: 'success' })
}

// ─── Gas tank balance ─────────────────────────────────────────────────────────
const gasTankAddress = computed(() => project.value?.gasTankAddress as `0x${string}` | undefined)
const { data: tankBalance, refetch: refetchBalance } = useBalance({
  address: gasTankAddress,
  chainId: 84532,
})
const tankEth = computed(() => {
  if (!tankBalance.value?.value) return null
  return Number(formatEther(tankBalance.value.value)).toFixed(4)
})
const tankColor = computed(() => {
  const v = Number(tankEth.value)
  if (!tankEth.value) return 'neutral'
  if (v === 0) return 'error'
  if (v < 0.01) return 'warning'
  return 'success'
})
</script>

<template>
  <div class="space-y-8">
    <!-- Back link + header -->
    <div>
      <NuxtLink to="/projects" class="inline-flex items-center gap-1 text-xs text-dimmed hover:text-muted mb-4 transition-colors">
        <UIcon name="i-heroicons-arrow-left" class="w-3.5 h-3.5" />
        All Projects
      </NuxtLink>

      <div v-if="pending" class="h-8 w-48 bg-elevated rounded-lg animate-pulse" />
      <div v-else-if="project" class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <h2 class="text-xl font-bold text-highlighted">{{ project.name }}</h2>
          <UBadge :color="project.active ? 'success' : 'neutral'" variant="subtle" size="sm">
            {{ project.active ? 'Active' : 'Paused' }}
          </UBadge>
          <span class="text-xs text-dimmed bg-elevated px-2 py-0.5 rounded-md">
            {{ CONFIG.networkName(project.chainId) }}
          </span>
        </div>
      </div>
    </div>

    <div v-if="!pending && project" class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Left column: integration info -->
      <div class="lg:col-span-2 space-y-6">
        <!-- API Integration card -->
        <div class="rounded-2xl border border-default overflow-hidden">
          <div class="px-5 py-4 bg-muted border-b border-default flex items-center justify-between">
            <div class="flex items-center gap-2">
              <UIcon name="i-heroicons-key" class="w-4 h-4 text-muted" />
              <span class="text-sm font-semibold text-toned">Integration</span>
            </div>
            <UButton icon="i-heroicons-plus" label="New API Key" color="neutral" variant="ghost" size="xs" @click="showNewKey = true" />
          </div>

          <div class="p-5 space-y-4 bg-default">
            <div class="space-y-1.5">
              <p class="text-xs font-semibold text-dimmed uppercase tracking-wide">Forwarder Address</p>
              <div class="flex items-center gap-2">
                <code class="flex-1 text-xs font-mono bg-muted border border-default rounded-lg px-3 py-2 text-toned truncate">{{ project.forwarderAddress }}</code>
                <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyText(project.forwarderAddress)" />
              </div>
            </div>

            <div class="space-y-1.5">
              <p class="text-xs font-semibold text-dimmed uppercase tracking-wide">Relay Endpoint</p>
              <div class="flex items-center gap-2">
                <code class="flex-1 text-xs font-mono bg-muted border border-default rounded-lg px-3 py-2 text-primary truncate">POST /relay</code>
                <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyText('POST /relay')" />
              </div>
              <p class="text-xs text-dimmed">Include your API key as <code class="text-primary">Authorization: Bearer &lt;key&gt;</code> or <code class="text-primary">X-API-Key</code>.</p>
            </div>
          </div>
        </div>

        <!-- Spending limits card -->
        <div class="rounded-2xl border border-default overflow-hidden">
          <div class="px-5 py-4 bg-muted border-b border-default flex items-center gap-2">
            <UIcon name="i-heroicons-shield-check" class="w-4 h-4 text-muted" />
            <span class="text-sm font-semibold text-toned">Spending Limits</span>
          </div>

          <div class="p-5 space-y-5 bg-default">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div class="space-y-1.5">
                <label class="text-xs font-semibold text-dimmed uppercase tracking-wide">Max Gas Per Request</label>
                <UInput v-model="limitsForm.maxGasPerRequest" placeholder="500000" size="sm" />
                <p class="text-xs text-dimmed">Gas units cap per relay call</p>
              </div>

              <div class="space-y-1.5">
                <label class="text-xs font-semibold text-dimmed uppercase tracking-wide">Max Gas Price (Gwei)</label>
                <UInput v-model="limitsForm.maxGasPriceGwei" type="number" placeholder="150" size="sm" />
                <p class="text-xs text-dimmed">Reject if gas price exceeds this</p>
              </div>

              <div class="space-y-1.5">
                <label class="text-xs font-semibold text-dimmed uppercase tracking-wide">Daily Gas Quota / User</label>
                <UInput v-model="limitsForm.dailyGasQuotaPerUser" placeholder="5000000" size="sm" />
                <p class="text-xs text-dimmed">Gas units per user per day (empty = unlimited)</p>
              </div>

              <div class="space-y-1.5">
                <label class="text-xs font-semibold text-dimmed uppercase tracking-wide">Rate Limit (req/min)</label>
                <UInput v-model="limitsForm.rateLimitPerMinute" type="number" placeholder="10" size="sm" />
                <p class="text-xs text-dimmed">Per-user rate limit per minute</p>
              </div>
            </div>

            <div class="space-y-1.5">
              <label class="text-xs font-semibold text-dimmed uppercase tracking-wide">Webhook URL (optional)</label>
              <UInput v-model="limitsForm.webhookUrl" placeholder="https://your-app.com/webhook" size="sm" />
              <p class="text-xs text-dimmed">Notified on relay success / failure</p>
            </div>

            <div class="flex justify-end">
              <UButton
                color="primary"
                label="Save Limits"
                size="sm"
                class="font-semibold"
                :loading="savingLimits"
                @click="saveLimits"
              />
            </div>
          </div>
        </div>
      </div>

      <!-- Right column: addresses -->
      <div class="space-y-4">
        <!-- Gas Tank -->
        <div class="rounded-2xl border border-default overflow-hidden">
          <div class="px-5 py-4 bg-muted border-b border-default flex items-center gap-2">
            <UIcon name="i-heroicons-circle-stack" class="w-4 h-4 text-muted" />
            <span class="text-sm font-semibold text-toned">Gas Tank</span>
          </div>
          <div class="p-5 bg-default space-y-3">
            <p class="text-xs text-dimmed">Fund this address with ETH. The relayer draws from it to pay gas on behalf of your users.</p>
            <div v-if="project.gasTankAddress" class="flex items-center gap-2">
              <code class="flex-1 text-xs font-mono bg-muted border border-default rounded-lg px-3 py-2 text-toned truncate">{{ project.gasTankAddress }}</code>
              <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyText(project.gasTankAddress!)" />
            </div>
            <div v-else class="text-xs text-dimmed italic">Not provisioned</div>
            <div class="flex items-center justify-between pt-1">
              <span class="text-xs text-dimmed">Balance</span>
              <div class="flex items-center gap-1.5">
                <span v-if="tankEth !== null" class="text-xs font-semibold" :class="`text-${tankColor}`">
                  {{ tankEth }} ETH
                </span>
                <span v-else class="text-xs text-dimmed">—</span>
                <UButton icon="i-heroicons-arrow-path" color="neutral" variant="ghost" size="xs" @click="refetchBalance()" />
              </div>
            </div>
          </div>
        </div>

        <!-- Relayer Wallet -->
        <div class="rounded-2xl border border-default overflow-hidden">
          <div class="px-5 py-4 bg-muted border-b border-default flex items-center gap-2">
            <UIcon name="i-heroicons-cpu-chip" class="w-4 h-4 text-muted" />
            <span class="text-sm font-semibold text-toned">Relayer Wallet</span>
          </div>
          <div class="p-5 bg-default space-y-3">
            <p class="text-xs text-dimmed">Hot wallet that submits transactions on-chain. Funded automatically from the gas tank.</p>
            <div v-if="project.relayerAddress" class="flex items-center gap-2">
              <code class="flex-1 text-xs font-mono bg-muted border border-default rounded-lg px-3 py-2 text-toned truncate">{{ project.relayerAddress }}</code>
              <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyText(project.relayerAddress!)" />
            </div>
          </div>
        </div>

        <!-- Project meta -->
        <div class="rounded-2xl border border-default p-5 bg-default space-y-3">
          <p class="text-xs font-semibold text-dimmed uppercase tracking-wide">Project Info</p>
          <div class="space-y-2 text-xs">
            <div class="flex justify-between">
              <span class="text-dimmed">Project ID</span>
              <code class="text-toned font-mono">{{ CONFIG.truncateAddress(project.id, 8, 6) }}</code>
            </div>
            <div class="flex justify-between">
              <span class="text-dimmed">Chain ID</span>
              <span class="text-toned font-medium">{{ project.chainId }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-dimmed">Created</span>
              <span class="text-toned">{{ new Date(project.createdAt).toLocaleDateString() }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- New API key modal -->
    <UModal v-model:open="showNewKey" title="Create API Key">
      <template #body>
        <div class="space-y-3 p-1">
          <div class="space-y-1.5">
            <label class="text-xs font-semibold text-muted uppercase tracking-wide">Key Name</label>
            <UInput v-model="newKeyName" placeholder="e.g. Production, Staging" size="md" autofocus @keydown.enter="createKey" />
          </div>
        </div>
      </template>
      <template #footer>
        <div class="flex justify-end gap-2 p-1">
          <UButton color="neutral" variant="outline" label="Cancel" @click="showNewKey = false" />
          <UButton color="primary" label="Create" class="font-semibold" :loading="creatingKey" :disabled="!newKeyName.trim()" @click="createKey" />
        </div>
      </template>
    </UModal>

    <!-- Key result modal — shown once -->
    <UModal v-model:open="showKeyResult" title="API Key Created" :dismissible="false">
      <template #body>
        <div class="space-y-4 p-1">
          <div class="p-3 rounded-lg bg-warning/10 border border-warning/20 flex gap-2">
            <UIcon name="i-heroicons-exclamation-triangle" class="w-4 h-4 text-warning shrink-0 mt-0.5" />
            <p class="text-xs text-warning">This key will <strong>not</strong> be shown again. Copy it now.</p>
          </div>
          <div v-if="newKeyResult" class="space-y-1.5">
            <p class="text-xs font-semibold text-muted uppercase tracking-wide">{{ newKeyResult.name }}</p>
            <div class="flex items-center gap-2">
              <code class="flex-1 text-xs font-mono bg-muted border border-default rounded-lg px-3 py-2.5 text-primary break-all">{{ newKeyResult.apiKey }}</code>
              <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyText(newKeyResult.apiKey)" />
            </div>
          </div>
        </div>
      </template>
      <template #footer>
        <div class="flex justify-end p-1">
          <UButton color="primary" label="Done" class="font-semibold" @click="showKeyResult = false" />
        </div>
      </template>
    </UModal>
  </div>
</template>
