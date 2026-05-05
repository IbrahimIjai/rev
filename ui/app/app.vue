<script setup lang="ts">
import { createAppKit } from '@reown/appkit/vue'
import { wagmiAdapter, networks, projectId } from './config/appkit'

createAppKit({
  adapters: [wagmiAdapter],
  networks,
  projectId,
  metadata: {
    name: 'GasRelayer',
    description: 'Gasless meta-transaction relayer powered by ERC-2771',
    url: 'https://gasrelayer.xyz',
    icons: ['https://avatars.githubusercontent.com/u/179229932?s=200&v=4'],
  },
})

</script>

<template>
  <client-only>
    <UApp>
      <NuxtLayout>
        <NuxtPage />
      </NuxtLayout>
    </UApp>
  </client-only>
</template>
