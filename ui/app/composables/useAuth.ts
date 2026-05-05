import { useAppKit, useAppKitAccount, useDisconnect } from '@reown/appkit/vue'
import { useSignMessage } from '@wagmi/vue'
import { useAuthStore } from '~/stores/useAuthStore'

export const useAuth = () => {
  const authStore = useAuthStore()
  const config = useRuntimeConfig()
  const toast = useToast()

  const { open } = useAppKit()
  // useAppKitAccount returns a ref — access nested state via .value
  const account = useAppKitAccount()
  const { signMessageAsync } = useSignMessage()
  const { disconnect } = useDisconnect()

  const loading = ref(false)
  const pendingSign = ref(false)

  const signIn = async () => {
    loading.value = true
    try {
      const addr = account.value.address?.toLowerCase()
      if (!addr) return

      const apiUrl = config.public.apiUrl as string
      const { nonce } = await $fetch<{ nonce: string }>(`${apiUrl}/api/auth/nonce?address=${addr}`)

      const message = [
        'Sign in to GasRelayer',
        '',
        `Address: ${addr}`,
        `Nonce: ${nonce}`,
        `Issued At: ${new Date().toISOString()}`,
      ].join('\n')

      const signature = await signMessageAsync({ message })

      const { token, business_id } = await $fetch<{ token: string; business_id: string }>(
        `${apiUrl}/api/auth/verify`,
        { method: 'POST', body: { address: addr, message, signature } },
      )

      authStore.setAuth(addr, token, business_id)
      toast.add({
        title: 'Connected',
        description: `${addr.slice(0, 6)}...${addr.slice(-4)}`,
        color: 'success',
      })
    }
    catch (err: unknown) {
      const msg =
        (err as { data?: { error?: string } })?.data?.error ??
        (err as { message?: string })?.message ??
        'Connection failed'
      const silent = ['user closed', 'cancelled', 'abort', 'rejected'].some(s =>
        msg.toLowerCase().includes(s),
      )
      if (!silent) {
        toast.add({ title: 'Failed to connect', description: msg, color: 'error' })
      }
      await disconnect()
    }
    finally {
      loading.value = false
      pendingSign.value = false
    }
  }

  watch(() => account.value.isConnected, async (connected) => {
    if (connected && pendingSign.value) {
      await signIn()
    }
  })

  const login = async () => {
    if (account.value.isConnected) {
      await signIn()
    }
    else {
      pendingSign.value = true
      open()
    }
  }

  const logout = async () => {
    loading.value = true
    try {
      await disconnect()
    }
    catch {}
    finally {
      authStore.logout()
      loading.value = false
      toast.add({ title: 'Disconnected', color: 'neutral' })
    }
  }

  return { login, logout, loading: computed(() => loading.value) }
}
