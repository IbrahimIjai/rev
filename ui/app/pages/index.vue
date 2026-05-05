<script setup lang="ts">
import { CONFIG } from '~/lib/config'
import type { ProjectSummary } from '~/composables/useApi'

const { isConnected, login, isLoading } = useSafe()
const api = useApi()

const { data: projects, pending, refresh } = useAsyncData(
  'projects',
  () => api.projects.list(),
  { watch: [isConnected], default: () => [] as ProjectSummary[] },
)

const stats = computed(() => [
  {
    label: 'Projects',
    value: String(projects.value?.length ?? 0),
    icon: 'i-heroicons-rectangle-stack',
    color: 'text-blue-500',
    bg: 'bg-blue-50 dark:bg-blue-500/10',
  },
  {
    label: 'Active Tanks',
    value: String(projects.value?.filter(p => p.active).length ?? 0),
    icon: 'i-heroicons-circle-stack',
    color: 'text-orange-500',
    bg: 'bg-orange-50 dark:bg-orange-500/10',
  },
  {
    label: 'Networks',
    value: String(new Set(projects.value?.map(p => p.chainId)).size ?? 0),
    icon: 'i-heroicons-globe-alt',
    color: 'text-green-500',
    bg: 'bg-green-50 dark:bg-green-500/10',
  },
  {
    label: 'API Keys',
    value: String(projects.value?.length ?? 0),
    icon: 'i-heroicons-key',
    color: 'text-purple-500',
    bg: 'bg-purple-50 dark:bg-purple-500/10',
  },
])
</script>

<template>
  <div class="w-full">
    <!-- Landing — not connected -->
    <div v-if="!isConnected" class="flex flex-col items-center text-center gap-10 py-20">
      <div class="inline-flex items-center gap-2 px-4 py-1.5 rounded-full bg-orange-50 dark:bg-orange-500/10 text-orange-600 dark:text-orange-400 text-xs font-semibold border border-orange-200 dark:border-orange-500/20">
        <UIcon name="i-heroicons-sparkles" class="w-3.5 h-3.5" />
        ERC-2771 Meta-Transactions
      </div>

      <div class="space-y-4 max-w-3xl">
        <h1 class="text-5xl sm:text-6xl font-black tracking-tight text-gray-900 dark:text-white leading-tight">
          Gasless experiences<br>
          <span class="text-transparent bg-clip-text bg-linear-to-r from-orange-500 to-amber-400">for every dApp</span>
        </h1>
        <p class="text-base text-gray-500 dark:text-gray-400 max-w-xl mx-auto leading-relaxed">
          Sponsor gas fees for your users in minutes. Connect, create a project, fund a tank, and ship gasless UX — no infrastructure required.
        </p>
      </div>

      <div class="flex flex-col sm:flex-row items-center gap-3">
        <UButton
          size="lg"
          color="primary"
          icon="i-heroicons-wallet"
          label="Connect to get started"
          class="font-semibold px-6"
          :loading="isLoading"
          @click="login"
        />
        <UButton
          size="lg"
          color="neutral"
          variant="outline"
          label="Read the docs"
        />
      </div>

      <!-- Feature pills -->
      <div class="flex flex-wrap justify-center gap-3 pt-8 border-t border-gray-100 dark:border-gray-900 w-full max-w-2xl">
        <div v-for="f in ['OZ ERC-2771 Forwarder', 'Multi-tenant projects', 'API key auth', 'Spending limits', 'Gas tank funding']" :key="f"
          class="px-3 py-1.5 bg-gray-50 dark:bg-gray-900 text-xs text-gray-500 dark:text-gray-400 rounded-full border border-gray-200 dark:border-gray-800 font-medium">
          {{ f }}
        </div>
      </div>
    </div>

    <!-- Dashboard — connected -->
    <div v-else class="space-y-8">
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-xl font-bold text-gray-900 dark:text-white">Overview</h2>
          <p class="text-sm text-gray-500 mt-0.5">Your gas relayer at a glance</p>
        </div>
        <NuxtLink to="/projects">
          <UButton size="sm" color="primary" icon="i-heroicons-plus" label="New Project" class="font-semibold" />
        </NuxtLink>
      </div>

      <!-- Stats -->
      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <div
          v-for="stat in stats"
          :key="stat.label"
          class="p-5 rounded-2xl border border-gray-100 dark:border-gray-900 bg-white dark:bg-gray-950 hover:border-orange-200 dark:hover:border-orange-500/20 transition-colors"
        >
          <div :class="[stat.bg, stat.color, 'w-9 h-9 rounded-xl flex items-center justify-center mb-4']">
            <UIcon :name="stat.icon" class="w-4.5 h-4.5" />
          </div>
          <p class="text-2xl font-black text-gray-900 dark:text-white tabular-nums">{{ stat.value }}</p>
          <p class="text-xs text-gray-400 mt-0.5">{{ stat.label }}</p>
        </div>
      </div>

      <!-- Projects list preview -->
      <div class="space-y-3">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-semibold text-gray-900 dark:text-white">Recent Projects</h3>
          <NuxtLink to="/projects" class="text-xs text-orange-500 hover:text-orange-600 font-medium">View all →</NuxtLink>
        </div>

        <div v-if="pending" class="space-y-3">
          <div v-for="i in 3" :key="i" class="h-16 rounded-xl bg-gray-100 dark:bg-gray-900 animate-pulse" />
        </div>

        <div v-else-if="!projects?.length" class="py-12 flex flex-col items-center gap-4 rounded-2xl border border-dashed border-gray-200 dark:border-gray-800">
          <UIcon name="i-heroicons-rectangle-stack" class="w-8 h-8 text-gray-300 dark:text-gray-700" />
          <div class="text-center">
            <p class="text-sm font-medium text-gray-500">No projects yet</p>
            <p class="text-xs text-gray-400 mt-0.5">Create your first project to start relaying</p>
          </div>
          <NuxtLink to="/projects">
            <UButton size="sm" color="primary" icon="i-heroicons-plus" label="Create project" class="font-semibold" />
          </NuxtLink>
        </div>

        <div v-else class="divide-y divide-gray-100 dark:divide-gray-900 rounded-2xl border border-gray-100 dark:border-gray-900 overflow-hidden">
          <NuxtLink
            v-for="p in projects?.slice(0, 5)"
            :key="p.id"
            :to="`/projects/${p.id}`"
            class="flex items-center justify-between px-5 py-4 bg-white dark:bg-gray-950 hover:bg-gray-50 dark:hover:bg-gray-900 transition-colors"
          >
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-lg bg-orange-50 dark:bg-orange-500/10 flex items-center justify-center">
                <UIcon name="i-heroicons-rectangle-stack" class="w-4 h-4 text-orange-500" />
              </div>
              <div>
                <p class="text-sm font-semibold text-gray-900 dark:text-white">{{ p.name }}</p>
                <p class="text-xs text-gray-400">{{ CONFIG.networkName(p.chainId) }}</p>
              </div>
            </div>
            <div class="flex items-center gap-3">
              <UBadge :color="p.active ? 'success' : 'neutral'" variant="subtle" size="xs">
                {{ p.active ? 'Active' : 'Paused' }}
              </UBadge>
              <UIcon name="i-heroicons-chevron-right" class="w-4 h-4 text-gray-300" />
            </div>
          </NuxtLink>
        </div>
      </div>
    </div>
  </div>
</template>
