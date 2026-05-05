<script setup lang="ts">
import { CONFIG } from '~/lib/config'
import type { ProjectDetail } from '~/composables/useApi'

const { isConnected } = useSafe()
const api = useApi()
const toast = useToast()

// Fetch all projects then derive tanks from them
const { data: projects, pending } = useAsyncData(
  'tanks-projects',
  async () => {
    const list = await api.projects.list()
    const details = await Promise.all(list.map(p => api.projects.get(p.id)))
    return details
  },
  { watch: [isConnected], default: () => [] as ProjectDetail[] },
)

const tanks = computed(() =>
  (projects.value ?? [])
    .filter(p => p.gasTankAddress)
    .map(p => ({
      projectId: p.id,
      projectName: p.name,
      address: p.gasTankAddress!,
      chainId: p.chainId,
      active: p.active,
    })),
)

async function copyAddress(addr: string) {
  await navigator.clipboard.writeText(addr)
  toast.add({ title: 'Address copied', color: 'success' })
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-xl font-bold text-gray-900 dark:text-white">Gas Tanks</h2>
        <p class="text-sm text-gray-500 mt-0.5">One tank per project — fund these addresses to sponsor gas for your users.</p>
      </div>
      <NuxtLink to="/projects">
        <UButton icon="i-heroicons-plus" label="New Project" color="primary" size="sm" class="font-semibold" />
      </NuxtLink>
    </div>

    <div v-if="!isConnected" class="py-20 flex flex-col items-center gap-4">
      <UIcon name="i-heroicons-lock-closed" class="w-10 h-10 text-gray-200 dark:text-gray-800" />
      <p class="text-sm text-gray-500">Connect your wallet to view gas tanks</p>
    </div>

    <div v-else-if="pending" class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div v-for="i in 3" :key="i" class="h-44 rounded-2xl bg-gray-100 dark:bg-gray-900 animate-pulse" />
    </div>

    <div v-else-if="!tanks.length" class="py-20 flex flex-col items-center gap-5 rounded-2xl border border-dashed border-gray-200 dark:border-gray-800">
      <div class="p-4 bg-orange-50 dark:bg-orange-500/10 rounded-2xl">
        <UIcon name="i-heroicons-circle-stack" class="w-8 h-8 text-orange-500" />
      </div>
      <div class="text-center">
        <p class="text-sm font-semibold text-gray-700 dark:text-gray-300">No gas tanks yet</p>
        <p class="text-xs text-gray-400 mt-1">Gas tanks are created automatically with each project.</p>
      </div>
      <NuxtLink to="/projects">
        <UButton icon="i-heroicons-plus" label="Create a project" color="primary" size="sm" class="font-semibold" />
      </NuxtLink>
    </div>

    <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div
        v-for="tank in tanks"
        :key="tank.address"
        class="rounded-2xl border border-gray-100 dark:border-gray-900 bg-white dark:bg-gray-950 overflow-hidden"
      >
        <!-- Status bar -->
        <div :class="tank.active ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-700'" class="h-1 w-full" />

        <div class="p-5 space-y-4">
          <!-- Header -->
          <div class="flex items-start justify-between">
            <div>
              <p class="text-sm font-semibold text-gray-900 dark:text-white">{{ tank.projectName }}</p>
              <p class="text-xs text-gray-400 mt-0.5">{{ CONFIG.networkName(tank.chainId) }} · Chain {{ tank.chainId }}</p>
            </div>
            <UBadge :color="tank.active ? 'success' : 'neutral'" variant="subtle" size="xs">
              {{ tank.active ? 'Active' : 'Paused' }}
            </UBadge>
          </div>

          <!-- Address -->
          <div class="space-y-1.5">
            <p class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Tank Address</p>
            <div class="flex items-center gap-2">
              <code class="flex-1 text-xs font-mono bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-lg px-3 py-2 text-gray-600 dark:text-gray-400 truncate">{{ tank.address }}</code>
              <UButton icon="i-heroicons-document-duplicate" color="neutral" variant="ghost" size="xs" @click="copyAddress(tank.address)" />
            </div>
          </div>

          <!-- Actions -->
          <div class="flex items-center gap-2 pt-1">
            <NuxtLink :to="`/projects/${tank.projectId}`" class="flex-1">
              <UButton block color="neutral" variant="outline" size="sm" label="View Project" />
            </NuxtLink>
            <UButton color="primary" size="sm" label="Fund Tank" class="font-semibold" @click="copyAddress(tank.address)" />
          </div>
        </div>
      </div>
    </div>

    <!-- Funding instructions -->
    <div v-if="tanks.length" class="p-5 rounded-2xl border border-gray-100 dark:border-gray-900 bg-gray-50 dark:bg-gray-900/50 flex gap-3">
      <UIcon name="i-heroicons-information-circle" class="w-5 h-5 text-blue-500 shrink-0 mt-0.5" />
      <div class="text-sm text-gray-600 dark:text-gray-400 space-y-1">
        <p class="font-medium text-gray-800 dark:text-gray-200">How to fund a tank</p>
        <p class="text-xs">Send ETH directly to the tank address from any wallet. The relayer will automatically draw from the tank balance to cover gas fees for your users' transactions.</p>
      </div>
    </div>
  </div>
</template>
