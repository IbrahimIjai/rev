<script setup lang="ts">
import { CONFIG } from '~/lib/config'

defineOptions({
  name: 'AppHeader'
})

const route = useRoute()
const colorMode = useColorMode()

const links = [
  { label: 'Overview', to: '/' },
  { label: 'Projects', to: '/projects' },
  { label: 'Gas Tanks', to: '/tanks' },
  { label: 'Settings', to: '/settings' }
]

const isDarkMode = computed({
  get() {
    return colorMode.value === 'dark'
  },
  set(value: boolean) {
    colorMode.preference = value ? 'dark' : 'light'
  }
})
const themeIcon = computed(() => isDarkMode.value ? 'i-heroicons-moon-20-solid' : 'i-heroicons-sun-20-solid')
const themeLabel = computed(() => `Switch to ${isDarkMode.value ? 'light' : 'dark'} mode`)

function toggleTheme() {
  isDarkMode.value = !isDarkMode.value
}
</script>

<template>
  <header
    class="h-16 border-b border-default bg-default/80 backdrop-blur-xl sticky top-0 z-50 flex items-center justify-between px-8 gap-4"
  >
    <NuxtLink
      to="/"
      class="flex items-center gap-2.5 shrink-0"
    >
      <div class="p-1.5 bg-primary rounded-lg shadow-sm">
        <UIcon
          :name="CONFIG.LOGO_ICON"
          class="w-4 h-4 text-inverted"
        />
      </div>
      <span class="text-sm font-black tracking-tight text-highlighted uppercase">{{ CONFIG.APP_NAME }}</span>
    </NuxtLink>

    <nav class="hidden md:flex items-center gap-0.5 absolute left-1/2 -translate-x-1/2">
      <NuxtLink
        v-for="link in links"
        :key="link.to"
        :to="link.to"
        class="text-sm font-semibold px-3.5 py-2 rounded-lg transition-all duration-150"
        :class="route.path === link.to || (link.to !== '/' && route.path.startsWith(link.to))
          ? 'text-highlighted bg-elevated'
          : 'text-muted hover:text-highlighted hover:bg-muted'"
      >
        {{ link.label }}
      </NuxtLink>
    </nav>

    <div class="flex items-center gap-2 shrink-0">
       <UColorModeButton />

      <ConnectButton />
    </div>
  </header>
</template>
