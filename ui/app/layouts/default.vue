<script setup lang="ts">
import AppFooter from '~/components/Footer.vue'
import AppHeader from '~/components/Header.vue'
</script>

<template>
  <div class="min-h-screen flex flex-col">
    <div
      class="fixed inset-0 -z-10 overflow-hidden pointer-events-none"
      aria-hidden="true"
    >
      <div class="absolute -left-56 top-[5%] w-225 h-225 rounded-full bg-primary/10 blur-[160px]" />
      <div class="absolute -right-64 bottom-[-15%] w-175 h-175 rounded-full bg-secondary/10 blur-[130px]" />
    </div>
    <AppHeader />

    <!-- Page content -->
    <main class="flex-1 w-full max-w-5xl mx-auto px-6 py-10">
      <slot />
    </main>

    <AppFooter />
  </div>
</template>
