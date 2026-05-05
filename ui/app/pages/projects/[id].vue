<script setup lang="ts">
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
</script>

<template>
  <div class="space-y-8">
    <!-- Back link + header -->
    <div>
      <NuxtLink to="/projects" class="inline-flex items-center gap-1 text-xs text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 mb-4 transition-colors">
        <UIcon name="i-heroicons-arrow-left" class="w-3.5 h-3.5" />
        All Projects
      </NuxtLink>

      <div v-if="pending" class="h-8 w-48 bg-gray-100 dark:bg-gray-900 rounded-lg animate-pulse" />
      <div v-else-if="project" class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <h2 class="text-xl font-bold text-gray-900 dark:text-white">{{ project.name }}</h2>
          <UBadge :color="project.active ? 'success' : 'neutral'" variant="subtle" size="sm">
            {{ project.active ? 'Active' : 'Paused' }}
          </UBadge>
          <span class="text-xs text-gray-400 bg-gray-100 dark:bg-gray-900 px-2 py-0.5 rounded-md">
            {{ CONFIG.networkName(project.chainId) }}
          </span>
        </div>
      </div>
    </div>

    <div v-if="!pending && project" class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Left column: integration info -->
      <div class="lg:col-span-2 space-y-6">
        <!-- API Integration card -->
        <div class="rounded-2xl border border-gray-100 dark:border-gray-900 overflow-hidden">
          <div class="px-5 py-4 bg-gray-50 dark:bg-gray-900/50 border-b border-gray-100 dark:border-gray-900 flex items-center justify-between">
            <div class="flex items-center gap-2">
              <UIcon name="i-heroicons-key" class="w-4 h-4 text-gray-500" />
              <span class="text-sm font-semibold text-gray-700 dark:text-gray-300">Integration</span>
            </div>
            <UButton icon="i-heroicons-plus" label="New API Key" color="neutral" variant="ghost" size="xs" @click="showNewKey = true" />
          </div>

          <div class="p-5 space-y-4 bg-white dark:bg-gray-950">
            <div class="space-y-1.5">
              <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Forwarder Address</p>
              <div class="flex items-center gap-2">
                <code class="flex-1 text-xs font-mono bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-lg px-3 py-2 text-gray-600 dark:text-gray-400 truncate">{{ project.forwarderAddress }}</code>
                <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyText(project.forwarderAddress)" />
              </div>
            </div>

            <div class="space-y-1.5">
              <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Relay Endpoint</p>
              <div class="flex items-center gap-2">
                <code class="flex-1 text-xs font-mono bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-lg px-3 py-2 text-orange-600 dark:text-orange-400 truncate">POST /relay</code>
                <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyText('POST /relay')" />
              </div>
              <p class="text-xs text-gray-400">Include your API key as <code class="text-orange-500">Authorization: Bearer &lt;key&gt;</code> or <code class="text-orange-500">X-API-Key</code>.</p>
            </div>
          </div>
        </div>

        <!-- Spending limits card -->
        <div class="rounded-2xl border border-gray-100 dark:border-gray-900 overflow-hidden">
          <div class="px-5 py-4 bg-gray-50 dark:bg-gray-900/50 border-b border-gray-100 dark:border-gray-900 flex items-center gap-2">
            <UIcon name="i-heroicons-shield-check" class="w-4 h-4 text-gray-500" />
            <span class="text-sm font-semibold text-gray-700 dark:text-gray-300">Spending Limits</span>
          </div>

          <div class="p-5 space-y-5 bg-white dark:bg-gray-950">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div class="space-y-1.5">
                <label class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Max Gas Per Request</label>
                <UInput v-model="limitsForm.maxGasPerRequest" placeholder="500000" size="sm" />
                <p class="text-xs text-gray-400">Gas units cap per relay call</p>
              </div>

              <div class="space-y-1.5">
                <label class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Max Gas Price (Gwei)</label>
                <UInput v-model="limitsForm.maxGasPriceGwei" type="number" placeholder="150" size="sm" />
                <p class="text-xs text-gray-400">Reject if gas price exceeds this</p>
              </div>

              <div class="space-y-1.5">
                <label class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Daily Gas Quota / User</label>
                <UInput v-model="limitsForm.dailyGasQuotaPerUser" placeholder="5000000" size="sm" />
                <p class="text-xs text-gray-400">Gas units per user per day (empty = unlimited)</p>
              </div>

              <div class="space-y-1.5">
                <label class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Rate Limit (req/min)</label>
                <UInput v-model="limitsForm.rateLimitPerMinute" type="number" placeholder="10" size="sm" />
                <p class="text-xs text-gray-400">Per-user rate limit per minute</p>
              </div>
            </div>

            <div class="space-y-1.5">
              <label class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Webhook URL (optional)</label>
              <UInput v-model="limitsForm.webhookUrl" placeholder="https://your-app.com/webhook" size="sm" />
              <p class="text-xs text-gray-400">Notified on relay success / failure</p>
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
        <div class="rounded-2xl border border-gray-100 dark:border-gray-900 overflow-hidden">
          <div class="px-5 py-4 bg-gray-50 dark:bg-gray-900/50 border-b border-gray-100 dark:border-gray-900 flex items-center gap-2">
            <UIcon name="i-heroicons-circle-stack" class="w-4 h-4 text-gray-500" />
            <span class="text-sm font-semibold text-gray-700 dark:text-gray-300">Gas Tank</span>
          </div>
          <div class="p-5 bg-white dark:bg-gray-950 space-y-3">
            <p class="text-xs text-gray-400">Fund this address with ETH. The relayer draws from it to pay gas on behalf of your users.</p>
            <div v-if="project.gasTankAddress" class="flex items-center gap-2">
              <code class="flex-1 text-xs font-mono bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-lg px-3 py-2 text-gray-600 dark:text-gray-400 truncate">{{ project.gasTankAddress }}</code>
              <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyText(project.gasTankAddress!)" />
            </div>
            <div v-else class="text-xs text-gray-400 italic">Not provisioned</div>
          </div>
        </div>

        <!-- Relayer Wallet -->
        <div class="rounded-2xl border border-gray-100 dark:border-gray-900 overflow-hidden">
          <div class="px-5 py-4 bg-gray-50 dark:bg-gray-900/50 border-b border-gray-100 dark:border-gray-900 flex items-center gap-2">
            <UIcon name="i-heroicons-cpu-chip" class="w-4 h-4 text-gray-500" />
            <span class="text-sm font-semibold text-gray-700 dark:text-gray-300">Relayer Wallet</span>
          </div>
          <div class="p-5 bg-white dark:bg-gray-950 space-y-3">
            <p class="text-xs text-gray-400">Hot wallet that submits transactions on-chain. Funded automatically from the gas tank.</p>
            <div v-if="project.relayerAddress" class="flex items-center gap-2">
              <code class="flex-1 text-xs font-mono bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-lg px-3 py-2 text-gray-600 dark:text-gray-400 truncate">{{ project.relayerAddress }}</code>
              <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyText(project.relayerAddress!)" />
            </div>
          </div>
        </div>

        <!-- Project meta -->
        <div class="rounded-2xl border border-gray-100 dark:border-gray-900 p-5 bg-white dark:bg-gray-950 space-y-3">
          <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Project Info</p>
          <div class="space-y-2 text-xs">
            <div class="flex justify-between">
              <span class="text-gray-400">Project ID</span>
              <code class="text-gray-600 dark:text-gray-400 font-mono">{{ CONFIG.truncateAddress(project.id, 8, 6) }}</code>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-400">Chain ID</span>
              <span class="text-gray-700 dark:text-gray-300 font-medium">{{ project.chainId }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-400">Created</span>
              <span class="text-gray-700 dark:text-gray-300">{{ new Date(project.createdAt).toLocaleDateString() }}</span>
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
            <label class="text-xs font-semibold text-gray-500 uppercase tracking-wide">Key Name</label>
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
          <div class="p-3 rounded-lg bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/20 flex gap-2">
            <UIcon name="i-heroicons-exclamation-triangle" class="w-4 h-4 text-amber-500 shrink-0 mt-0.5" />
            <p class="text-xs text-amber-700 dark:text-amber-400">This key will <strong>not</strong> be shown again. Copy it now.</p>
          </div>
          <div v-if="newKeyResult" class="space-y-1.5">
            <p class="text-xs font-semibold text-gray-500 uppercase tracking-wide">{{ newKeyResult.name }}</p>
            <div class="flex items-center gap-2">
              <code class="flex-1 text-xs font-mono bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-lg px-3 py-2.5 text-orange-600 dark:text-orange-400 break-all">{{ newKeyResult.apiKey }}</code>
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
