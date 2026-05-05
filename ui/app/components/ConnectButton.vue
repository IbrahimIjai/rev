<script setup lang="ts">
import { useAppKit } from '@reown/appkit/vue'
import { useAccount } from '@wagmi/vue'
import { CONFIG } from '~/lib/config'

const { login, logout, loading } = useAuth()
const { open } = useAppKit()
const { address, isConnected, isConnecting, isReconnecting } = useAccount()

const buttonLoading = computed(() => loading.value || isConnecting.value || isReconnecting.value)
const truncatedAddress = computed(() => CONFIG.truncateAddress(address.value ?? ''))

function openAccountDialog() {
  if (isConnected.value) {
    open({ view: 'Account' })
    return
  }

  login()
}
</script>

<template>
  <div class="flex items-center gap-2">
    <template v-if="isConnected">
      <UButton
        color="neutral"
        variant="soft"
        size="sm"
        class="font-mono font-bold"
        :label="truncatedAddress"
        @click="openAccountDialog"
      >
        <template #leading>
          <span class="w-2 h-2 rounded-full bg-success shrink-0" />
        </template>
        <template #trailing>
          <UIcon
            name="i-heroicons-chevron-down-20-solid"
            class="w-4 h-4 text-dimmed"
          />
        </template>
      </UButton>

      <UButton
        icon="i-heroicons-arrow-left-on-rectangle"
        color="neutral"
        variant="ghost"
        size="sm"
        :loading="loading"
        aria-label="Disconnect wallet"
        title="Disconnect wallet"
        @click="logout"
      />
    </template>

    <UButton
      v-else
      color="primary"
      size="md"
      :loading="buttonLoading"
      @click="login"
    >
      <template #leading>
        <UIcon
          name="i-heroicons-wallet-20-solid"
          class="w-4 h-4"
        />
      </template>
      Connect
      <template #trailing>
        <UIcon
          name="i-heroicons-arrow-right-20-solid"
          class="w-4 h-4"
        />
      </template>
    </UButton>
  </div>
</template>
