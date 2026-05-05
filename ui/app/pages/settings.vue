<script setup lang="ts">
const { isConnected, login, isLoading: authLoading } = useSafe()
const api = useApi()
const toast = useToast()
const authStore = useAuthStore()

// ─── API Keys ────────────────────────────────────────────────────────────────
// Keys are per-project. Settings page shows a quick-create form for any project.
const { data: projects, pending: loadingProjects } = useAsyncData(
  'settings-projects',
  () => api.projects.list(),
  { watch: [isConnected], default: () => [] },
)

const selectedProjectId = ref('')
const newKeyName = ref('')
const creatingKey = ref(false)
const newKeyResult = ref<{ apiKey: string; name: string } | null>(null)
const showKeyResult = ref(false)

watch(projects, (list) => {
  if (list?.length && !selectedProjectId.value) {
    selectedProjectId.value = list[0]!.id
  }
})

const projectOptions = computed(() =>
  (projects.value ?? []).map(p => ({ label: p.name, value: p.id })),
)

async function createKey() {
  if (!selectedProjectId.value || !newKeyName.value.trim()) return
  creatingKey.value = true
  try {
    const result = await api.projects.createApiKey(selectedProjectId.value, newKeyName.value.trim())
    newKeyResult.value = result
    newKeyName.value = ''
    showKeyResult.value = true
    toast.add({ title: 'API key created', color: 'success' })
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
  <div class="max-w-2xl mx-auto space-y-8">
    <div>
      <h2 class="text-xl font-bold text-gray-900 dark:text-white">Settings</h2>
      <p class="text-sm text-gray-500 mt-0.5">Manage your account and API credentials.</p>
    </div>

    <!-- Not connected -->
    <div v-if="!isConnected" class="py-16 flex flex-col items-center gap-4 rounded-2xl border border-dashed border-gray-200 dark:border-gray-800">
      <UIcon name="i-heroicons-lock-closed" class="w-8 h-8 text-gray-300 dark:text-gray-700" />
      <p class="text-sm text-gray-500">Connect your wallet to manage settings</p>
      <UButton icon="i-heroicons-wallet" label="Connect" color="primary" size="sm" :loading="authLoading" @click="login" />
    </div>

    <template v-else>
      <!-- Account info -->
      <div class="rounded-2xl border border-gray-100 dark:border-gray-900 overflow-hidden">
        <div class="px-5 py-4 bg-gray-50 dark:bg-gray-900/50 border-b border-gray-100 dark:border-gray-900 flex items-center gap-2">
          <UIcon name="i-heroicons-user-circle" class="w-4 h-4 text-gray-500" />
          <span class="text-sm font-semibold text-gray-700 dark:text-gray-300">Account</span>
        </div>
        <div class="p-5 bg-white dark:bg-gray-950 space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-xs text-gray-400">Wallet address</span>
            <code class="text-xs font-mono text-gray-700 dark:text-gray-300">{{ authStore.address }}</code>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-xs text-gray-400">Business ID</span>
            <code class="text-xs font-mono text-gray-500">{{ authStore.businessId?.slice(0, 12) }}...</code>
          </div>
        </div>
      </div>

      <!-- API Key generator -->
      <div class="rounded-2xl border border-gray-100 dark:border-gray-900 overflow-hidden">
        <div class="px-5 py-4 bg-gray-50 dark:bg-gray-900/50 border-b border-gray-100 dark:border-gray-900 flex items-center gap-2">
          <UIcon name="i-heroicons-key" class="w-4 h-4 text-gray-500" />
          <span class="text-sm font-semibold text-gray-700 dark:text-gray-300">Generate API Key</span>
        </div>

        <div class="p-5 bg-white dark:bg-gray-950 space-y-4">
          <p class="text-sm text-gray-500">API keys are scoped per project. Each key can relay transactions to the project's forwarder on the configured chain.</p>

          <div v-if="loadingProjects" class="h-9 bg-gray-100 dark:bg-gray-900 rounded-lg animate-pulse" />

          <div v-else-if="!projects?.length" class="text-sm text-gray-400 italic">
            No projects yet.
            <NuxtLink to="/projects" class="text-orange-500 hover:underline">Create one first.</NuxtLink>
          </div>

          <template v-else>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div class="space-y-1.5">
                <label class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Project</label>
                <USelect v-model="selectedProjectId" :items="projectOptions" size="sm" />
              </div>
              <div class="space-y-1.5">
                <label class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Key Name</label>
                <UInput v-model="newKeyName" placeholder="e.g. Production" size="sm" @keydown.enter="createKey" />
              </div>
            </div>

            <div class="flex justify-end">
              <UButton
                color="primary"
                label="Generate Key"
                size="sm"
                class="font-semibold"
                :loading="creatingKey"
                :disabled="!newKeyName.trim() || !selectedProjectId"
                @click="createKey"
              />
            </div>
          </template>
        </div>
      </div>

      <!-- Manage keys note -->
      <div class="p-4 rounded-xl bg-gray-50 dark:bg-gray-900/50 border border-gray-100 dark:border-gray-900 flex gap-3">
        <UIcon name="i-heroicons-information-circle" class="w-4 h-4 text-blue-500 shrink-0 mt-0.5" />
        <p class="text-xs text-gray-500">
          API keys are scoped to their project. To view all keys or revoke one, go to the
          <NuxtLink to="/projects" class="text-orange-500 hover:underline">project page</NuxtLink>.
        </p>
      </div>
    </template>

    <!-- New key result modal -->
    <UModal v-model:open="showKeyResult" title="API Key Created" :dismissible="false">
      <template #body>
        <div class="space-y-4 p-1">
          <div class="p-3 rounded-lg bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/20 flex gap-2">
            <UIcon name="i-heroicons-exclamation-triangle" class="w-4 h-4 text-amber-500 shrink-0 mt-0.5" />
            <p class="text-xs text-amber-700 dark:text-amber-400">Save this key now — it will <strong>not</strong> be shown again.</p>
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
