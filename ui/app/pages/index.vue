<script setup lang="ts">
import { CONFIG } from '~/lib/config'
import type { ProjectSummary } from '~/composables/useApi'

const authStore = useAuthStore()
const { login, loading } = useAuth()
const api = useApi()

const { data: projects, pending } = useAsyncData(
  'projects',
  () => api.projects.list(),
  { watch: [computed(() => authStore.isConnected)], default: () => [] as ProjectSummary[] },
)

const stats = computed(() => [
  {
    label: 'Projects',
    value: String(projects.value?.length ?? 0),
    icon: 'i-heroicons-rectangle-stack',
    color: 'text-info',
    bg: 'bg-info/10',
  },
  {
    label: 'Active Tanks',
    value: String(projects.value?.filter(p => p.active).length ?? 0),
    icon: 'i-heroicons-circle-stack',
    color: 'text-primary',
    bg: 'bg-primary/10',
  },
  {
    label: 'Networks',
    value: String(new Set(projects.value?.map(p => p.chainId)).size ?? 0),
    icon: 'i-heroicons-globe-alt',
    color: 'text-success',
    bg: 'bg-success/10',
  },
  {
    label: 'API Keys',
    value: String(projects.value?.length ?? 0),
    icon: 'i-heroicons-key',
    color: 'text-secondary',
    bg: 'bg-secondary/10',
  },
])
</script>

<template>
  <div class="w-full">

    <!-- ── Landing — not connected ───────────────────────────────────────── -->
    <div
      v-if="!authStore.isConnected"
      class="flex flex-col items-center justify-center text-center gap-10 min-h-[calc(100vh-4rem)] py-20"
    >
      <!-- Badge -->
      <div class="inline-flex items-center gap-2 px-4 py-1.5 rounded-full bg-default/80 text-toned text-xs font-semibold border border-default  shadow-sm backdrop-blur-sm">
        <div class="w-1.5 h-1.5 rounded-full bg-primary" />
        ERC-2771 Meta-Transactions
      </div>

      <!-- Headline -->
      <div class="space-y-5 max-w-4xl">
        <h1 class="text-6xl sm:text-7xl lg:text-8xl font-black tracking-tighter text-highlighted leading-[0.95]">
          Gasless experiences<br>
          <span class="text-primary">
            for every dApp
          </span>
        </h1>
        <p class="text-lg sm:text-xl text-muted max-w-xl mx-auto leading-relaxed font-medium">
          Sponsor gas fees for your users in minutes. Connect, create a project,
          fund a tank, and ship gasless UX — no infrastructure required.
        </p>
      </div>

      <!-- CTA row -->
      <div class="flex flex-col sm:flex-row items-center gap-3">
        <UButton
          size="xl"
          color="primary"
          class="font-bold px-8"
          :loading="loading"
          @click="login"
        >
          <template #leading>
            <UIcon name="i-heroicons-wallet" class="w-5 h-5" />
          </template>
          Connect to get started
          <template #trailing>
            <UIcon name="i-heroicons-arrow-right" class="w-4 h-4" />
          </template>
        </UButton>
        <UButton
          size="xl"
          color="neutral"
          variant="outline"
          label="Read the docs"
          class="font-semibold"
        />
      </div>

      <!-- Feature pills -->
      <div class="flex flex-wrap justify-center gap-2.5 pt-6 border-t border-default w-full max-w-2xl">
        <span
          v-for="f in ['OZ ERC-2771 Forwarder', 'Multi-tenant projects', 'API key auth', 'Spending limits', 'Gas tank funding', 'Arc Testnet']"
          :key="f"
          class="px-3.5 py-1.5 bg-default/60 text-xs text-muted rounded-full border border-default font-semibold backdrop-blur-sm"
        >
          {{ f }}
        </span>
      </div>
    </div>

    <!-- ── Dashboard — connected ─────────────────────────────────────────── -->
    <div v-else class="space-y-8">
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-xl font-black text-highlighted tracking-tight">Overview</h2>
          <p class="text-sm text-muted mt-0.5 font-medium">Your gas relayer at a glance</p>
        </div>
        <NuxtLink to="/projects">
          <UButton size="sm" color="primary" icon="i-heroicons-plus" label="New Project" class="font-bold" />
        </NuxtLink>
      </div>

      <!-- Stats -->
      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <div
          v-for="stat in stats"
          :key="stat.label"
          class="p-5 rounded-2xl border border-default bg-default/60 backdrop-blur-sm hover:border-primary/20 transition-colors"
        >
          <div :class="[stat.bg, stat.color, 'w-9 h-9 rounded-xl flex items-center justify-center mb-4']">
            <UIcon :name="stat.icon" class="w-4.5 h-4.5" />
          </div>
          <p class="text-2xl font-black text-highlighted tabular-nums tracking-tight">{{ stat.value }}</p>
          <p class="text-xs text-dimmed mt-0.5 font-semibold">{{ stat.label }}</p>
        </div>
      </div>

      <!-- Projects list preview -->
      <div class="space-y-3">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-black text-highlighted tracking-tight">Recent Projects</h3>
          <NuxtLink to="/projects" class="text-xs text-primary hover:text-primary font-bold">View all →</NuxtLink>
        </div>

        <div v-if="pending" class="space-y-3">
          <div v-for="i in 3" :key="i" class="h-16 rounded-xl bg-muted animate-pulse" />
        </div>

        <div v-else-if="!projects?.length" class="py-12 flex flex-col items-center gap-4 rounded-2xl border border-dashed border-default bg-default/40 backdrop-blur-sm">
          <UIcon name="i-heroicons-rectangle-stack" class="w-8 h-8 text-dimmed" />
          <div class="text-center">
            <p class="text-sm font-bold text-muted">No projects yet</p>
            <p class="text-xs text-dimmed mt-0.5 font-medium">Create your first project to start relaying</p>
          </div>
          <NuxtLink to="/projects">
            <UButton size="sm" color="primary" icon="i-heroicons-plus" label="Create project" class="font-bold" />
          </NuxtLink>
        </div>

        <div v-else class="divide-y divide-default rounded-2xl border border-default overflow-hidden bg-default/60 backdrop-blur-sm">
          <NuxtLink
            v-for="p in projects?.slice(0, 5)"
            :key="p.id"
            :to="`/projects/${p.id}`"
            class="flex items-center justify-between px-5 py-4 hover:bg-muted/80 transition-colors"
          >
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center">
                <UIcon name="i-heroicons-rectangle-stack" class="w-4 h-4 text-primary" />
              </div>
              <div>
                <p class="text-sm font-bold text-highlighted">{{ p.name }}</p>
                <p class="text-xs text-dimmed font-medium">{{ CONFIG.networkName(p.chainId) }}</p>
              </div>
            </div>
            <div class="flex items-center gap-3">
              <UBadge :color="p.active ? 'success' : 'neutral'" variant="subtle" size="xs">
                {{ p.active ? 'Active' : 'Paused' }}
              </UBadge>
              <UIcon name="i-heroicons-chevron-right" class="w-4 h-4 text-dimmed" />
            </div>
          </NuxtLink>
        </div>
      </div>
    </div>

  </div>
</template>
