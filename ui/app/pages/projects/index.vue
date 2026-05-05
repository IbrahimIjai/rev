<script setup lang="ts">
import { CONFIG } from '~/lib/config'
import type { CreateProjectResponse, ProjectSummary } from '~/composables/useApi'

const authStore = useAuthStore()
const { login, loading: authLoading } = useAuth()
const api = useApi()
const toast = useToast()

const { data: projects, pending, refresh } = useAsyncData(
  'projects-list',
  () => api.projects.list(),
  { watch: [computed(() => authStore.isConnected)], default: () => [] as ProjectSummary[] },
)

// ─── Create project modal ────────────────────────────────────────────────────
const showCreate = ref(false)
const creating = ref(false)
const newProject = reactive({ name: '', chainId: 1, forwarderAddress: '' })

// Newly created project result — shown once in a reveal modal
const createdResult = ref<CreateProjectResponse | null>(null)
const showResult = ref(false)

const chainOptions = Object.entries(CONFIG.NETWORKS).map(([id, n]) => ({
  label: n.name,
  value: Number(id),
}))

async function createProject() {
  if (!newProject.name || !newProject.forwarderAddress) return
  creating.value = true
  try {
    const result = await api.projects.create({
      name: newProject.name,
      chainId: newProject.chainId,
      forwarderAddress: newProject.forwarderAddress,
    })
    createdResult.value = result
    showCreate.value = false
    showResult.value = true
    newProject.name = ''
    newProject.forwarderAddress = ''
    await refresh()
  }
  catch (err: unknown) {
    const msg = (err as { data?: { error?: string } })?.data?.error ?? 'Failed to create project'
    toast.add({ title: 'Error', description: msg, color: 'error' })
  }
  finally {
    creating.value = false
  }
}

async function copyText(text: string, label: string) {
  await navigator.clipboard.writeText(text)
  toast.add({ title: `${label} copied`, color: 'success' })
}
</script>

<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-xl font-bold text-highlighted">Projects</h2>
        <p class="text-sm text-muted mt-0.5">Each project gets its own gas tank, relayer wallet, and API key.</p>
      </div>
      <UButton
        v-if="authStore.isConnected"
        icon="i-heroicons-plus"
        label="New Project"
        color="primary"
        size="sm"
        class="font-semibold"
        @click="showCreate = true"
      />
      <UButton
        v-else
        icon="i-heroicons-wallet"
        label="Connect"
        color="primary"
        size="sm"
        :loading="authLoading"
        @click="login"
      />
    </div>

    <!-- Not connected state -->
    <div v-if="!authStore.isConnected" class="py-20 flex flex-col items-center gap-4">
      <UIcon name="i-heroicons-lock-closed" class="w-10 h-10 text-dimmed" />
      <p class="text-sm text-muted">Connect your wallet to manage projects</p>
    </div>

    <!-- Loading skeleton -->
    <div v-else-if="pending" class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div v-for="i in 4" :key="i" class="h-40 rounded-2xl bg-elevated animate-pulse" />
    </div>

    <!-- Empty state -->
    <div v-else-if="!projects?.length" class="py-20 flex flex-col items-center gap-5 rounded-2xl border border-dashed border-default">
      <div class="p-4 bg-primary/10 rounded-2xl">
        <UIcon name="i-heroicons-rectangle-stack" class="w-8 h-8 text-primary" />
      </div>
      <div class="text-center">
        <p class="text-sm font-semibold text-toned">No projects yet</p>
        <p class="text-xs text-dimmed mt-1">Create a project to get your first API key and gas tank.</p>
      </div>
      <UButton icon="i-heroicons-plus" label="Create your first project" color="primary" size="sm" class="font-semibold" @click="showCreate = true" />
    </div>

    <!-- Project grid -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <NuxtLink
        v-for="p in projects"
        :key="p.id"
        :to="`/projects/${p.id}`"
        class="group p-5 rounded-2xl border border-default bg-default hover:border-primary/20 hover:shadow-md transition-all duration-200 block"
      >
        <div class="flex items-start justify-between mb-4">
          <div class="flex items-center gap-3">
            <div class="w-9 h-9 rounded-xl bg-primary/10 flex items-center justify-center shrink-0">
              <UIcon name="i-heroicons-rectangle-stack" class="w-4.5 h-4.5 text-primary" />
            </div>
            <div>
              <p class="text-sm font-semibold text-highlighted group-hover:text-primary transition-colors">{{ p.name }}</p>
              <p class="text-xs text-dimmed">{{ CONFIG.networkName(p.chainId) }}</p>
            </div>
          </div>
          <UBadge :color="p.active ? 'success' : 'neutral'" variant="subtle" size="xs">
            {{ p.active ? 'Active' : 'Paused' }}
          </UBadge>
        </div>

        <div class="space-y-2 text-xs text-dimmed">
          <div class="flex items-center gap-2">
            <UIcon name="i-heroicons-calendar" class="w-3.5 h-3.5 shrink-0" />
            Created {{ new Date(p.createdAt).toLocaleDateString() }}
          </div>
          <div class="flex items-center gap-2 font-mono">
            <UIcon name="i-heroicons-cpu-chip" class="w-3.5 h-3.5 shrink-0" />
            <span class="truncate">{{ CONFIG.truncateAddress(p.forwarderAddress, 10, 8) }}</span>
          </div>
        </div>

        <div class="mt-4 pt-4 border-t border-default flex items-center justify-between">
          <span class="text-xs text-dimmed">Chain {{ p.chainId }}</span>
          <span class="text-xs text-primary font-medium group-hover:underline">View project →</span>
        </div>
      </NuxtLink>
    </div>

    <!-- Create project modal -->
    <UModal v-model:open="showCreate" title="New Project">
      <template #body>
        <div class="space-y-4 p-1">
          <div class="space-y-1.5">
            <label class="text-xs font-semibold text-muted uppercase tracking-wide">Project Name</label>
            <UInput v-model="newProject.name" placeholder="My dApp" size="md" autofocus />
          </div>

          <div class="space-y-1.5">
            <label class="text-xs font-semibold text-muted uppercase tracking-wide">Network</label>
            <USelect v-model="newProject.chainId" :items="chainOptions" size="md" />
          </div>

          <div class="space-y-1.5">
            <label class="text-xs font-semibold text-muted uppercase tracking-wide">ERC2771Forwarder Address</label>
            <UInput v-model="newProject.forwarderAddress" placeholder="0x..." size="md" font-mono class="font-mono" />
            <p class="text-xs text-dimmed">The deployed OZ ERC2771Forwarder contract on this network.</p>
          </div>
        </div>
      </template>
      <template #footer>
        <div class="flex justify-end gap-2 p-1">
          <UButton color="neutral" variant="outline" label="Cancel" @click="showCreate = false" />
          <UButton
            color="primary"
            label="Create Project"
            class="font-semibold"
            :loading="creating"
            :disabled="!newProject.name || !newProject.forwarderAddress"
            @click="createProject"
          />
        </div>
      </template>
    </UModal>

    <!-- Created result modal — API key shown ONCE -->
    <UModal v-model:open="showResult" title="Project Created" :dismissible="false">
      <template #body>
        <div class="space-y-4 p-1">
          <div class="p-3 rounded-lg bg-warning/10 border border-warning/20 flex gap-2">
            <UIcon name="i-heroicons-exclamation-triangle" class="w-4 h-4 text-warning shrink-0 mt-0.5" />
            <p class="text-xs text-warning">Save your API key now — it will <strong>never be shown again</strong>.</p>
          </div>

          <div v-if="createdResult" class="space-y-3">
            <div v-for="field in [
              { label: 'API Key', value: createdResult.apiKey, mono: true },
              { label: 'Gas Tank Address', value: createdResult.gasTankAddress, mono: true },
              { label: 'Relayer Address', value: createdResult.relayerAddress, mono: true },
            ]" :key="field.label" class="space-y-1">
              <p class="text-xs font-semibold text-muted uppercase tracking-wide">{{ field.label }}</p>
              <div class="flex items-center gap-2">
                <code class="flex-1 text-xs font-mono bg-muted border border-default rounded-lg px-3 py-2 text-toned truncate">{{ field.value }}</code>
                <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyText(field.value, field.label)" />
              </div>
            </div>
          </div>
        </div>
      </template>
      <template #footer>
        <div class="flex justify-end p-1">
          <UButton
            color="primary"
            label="I've saved the API key"
            class="font-semibold"
            @click="showResult = false; navigateTo(`/projects/${createdResult?.projectId}`)"
          />
        </div>
      </template>
    </UModal>
  </div>
</template>
