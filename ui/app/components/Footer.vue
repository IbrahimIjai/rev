<script setup lang="ts">
import { CONFIG } from '~/lib/config'

defineOptions({
  name: 'AppFooter'
})
</script>

<template>
  <footer class="border-t border-default">
    <div class="max-w-5xl mx-auto px-6 py-6 flex flex-col sm:flex-row items-center justify-between gap-4">
      <div class="flex items-center gap-2">
        <UIcon
          :name="CONFIG.LOGO_ICON"
          class="w-4 h-4 text-primary"
        />
        <span class="text-xs font-black text-muted uppercase tracking-tight">{{ CONFIG.APP_NAME }}</span>
      </div>
      <div class="flex items-center gap-6 text-xs text-muted">
        <a
          href="#"
          class="hover:text-primary transition-colors font-medium"
        >Docs</a>
        <a
          href="#"
          class="hover:text-primary transition-colors font-medium"
        >Support</a>
        <a
          href="#"
          class="hover:text-primary transition-colors font-medium"
        >Status</a>
      </div>
      <p class="text-xs text-muted font-medium">
        © {{ new Date().getFullYear() }} GasRelayer
      </p>
    </div>
  </footer>
</template>
