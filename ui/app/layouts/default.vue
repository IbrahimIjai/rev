<script setup lang="ts">
import { CONFIG } from '~/lib/config'

const { login, isConnected, safeAddress, isLoading, logout } = useSafe()
const route = useRoute()

const links = [
  { label: 'Overview', to: '/' },
  { label: 'Projects', to: '/projects' },
  { label: 'Gas Tanks', to: '/tanks' },
  { label: 'Settings', to: '/settings' },
]
</script>

<template>
  <div class="min-h-screen bg-white dark:bg-gray-950 flex flex-col">
    <header class="h-14 border-b border-gray-100 dark:border-gray-900 bg-white/80 dark:bg-gray-950/80 backdrop-blur-md sticky top-0 z-50 flex items-center justify-between px-6">
      <NuxtLink to="/" class="flex items-center gap-2 shrink-0">
        <div class="p-1.5 bg-orange-500 rounded-lg shadow-sm shadow-orange-500/30">
          <UIcon :name="CONFIG.LOGO_ICON" class="w-4 h-4 text-white" />
        </div>
        <span class="text-sm font-black tracking-tight text-gray-900 dark:text-white uppercase">{{ CONFIG.APP_NAME }}</span>
      </NuxtLink>

      <nav class="hidden md:flex items-center gap-6">
        <NuxtLink
          v-for="link in links"
          :key="link.to"
          :to="link.to"
          class="text-sm font-medium transition-colors duration-150"
          :class="route.path === link.to || (link.to !== '/' && route.path.startsWith(link.to))
            ? 'text-orange-600 dark:text-orange-400'
            : 'text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200'"
        >
          {{ link.label }}
        </NuxtLink>
      </nav>

      <div class="flex items-center gap-3">
        <UColorModeButton size="sm" variant="ghost" color="neutral" />

        <template v-if="isConnected">
          <div class="hidden sm:flex items-center gap-2 px-3 py-1.5 bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-800">
            <div class="w-1.5 h-1.5 rounded-full bg-green-500" />
            <span class="text-xs font-mono text-gray-600 dark:text-gray-400">{{ CONFIG.truncateAddress(safeAddress ?? '') }}</span>
          </div>
          <UButton
            icon="i-heroicons-arrow-left-on-rectangle"
            color="neutral"
            variant="ghost"
            size="sm"
            :loading="isLoading"
            @click="logout"
          />
        </template>

        <UButton
          v-else
          icon="i-heroicons-wallet"
          label="Connect"
          color="primary"
          size="sm"
          class="font-semibold"
          :loading="isLoading"
          @click="login"
        />
      </div>
    </header>

    <main class="flex-1 w-full max-w-5xl mx-auto px-6 py-10">
      <slot />
    </main>

    <footer class="border-t border-gray-100 dark:border-gray-900">
      <div class="max-w-5xl mx-auto px-6 py-6 flex flex-col sm:flex-row items-center justify-between gap-4">
        <div class="flex items-center gap-2 opacity-50">
          <UIcon :name="CONFIG.LOGO_ICON" class="w-4 h-4 text-orange-500" />
          <span class="text-xs font-bold text-gray-700 dark:text-gray-300">{{ CONFIG.APP_NAME }}</span>
        </div>
        <div class="flex items-center gap-6 text-xs text-gray-400">
          <a href="#" class="hover:text-orange-500 transition-colors">Docs</a>
          <a href="#" class="hover:text-orange-500 transition-colors">Support</a>
          <a href="#" class="hover:text-orange-500 transition-colors">Status</a>
        </div>
        <p class="text-xs text-gray-400">© {{ new Date().getFullYear() }} GasRelayer</p>
      </div>
    </footer>
  </div>
</template>
