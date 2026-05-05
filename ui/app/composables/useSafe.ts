import { useWeb3AuthConnect, useWeb3Auth, useWeb3AuthDisconnect } from '@web3auth/modal/vue'
import { useAuthStore } from '~/stores/useAuthStore'

export const useSafe = () => {
  const authStore = useAuthStore()
  const runtimeConfig = useRuntimeConfig()
  const toast = useToast()

  // Official Vue SDK composables — must be called inside component/composable setup
  const { connect: w3aConnect, loading: connectLoading } = useWeb3AuthConnect()
  const { provider } = useWeb3Auth()
  const { disconnect: w3aDisconnect } = useWeb3AuthDisconnect()

  const actionLoading = ref(false)
  // Expose a combined loading state
  const isLoading = computed(() => actionLoading.value || connectLoading.value)

  const login = async () => {
    actionLoading.value = true
    try {
      // connect() shows the Web3Auth modal — returns EIP-1193 provider on success
      const prov = await w3aConnect()
      if (!prov) return // user closed the modal

      const accounts = (await prov.request({ method: 'eth_accounts' })) as string[]
      const address = (accounts[0] ?? '').toLowerCase()
      if (!address) throw new Error('No account returned from wallet')

      const apiUrl = runtimeConfig.public.apiUrl as string

      // 1. Get nonce from backend
      const { nonce } = await $fetch<{ nonce: string }>(`${apiUrl}/api/auth/nonce?address=${address}`)

      // 2. Build message — backend checks that nonce is contained in the message
      const message = [
        'Sign in to GasRelayer',
        '',
        `Address: ${address}`,
        `Nonce: ${nonce}`,
        `Issued At: ${new Date().toISOString()}`,
      ].join('\n')

      // 3. personal_sign — wallet prefixes with \x19Ethereum Signed Message:\n{len}
      const signature = (await prov.request({
        method: 'personal_sign',
        params: [message, address],
      })) as string

      // 4. Verify with backend → JWT
      const { token, business_id } = await $fetch<{ token: string; business_id: string }>(
        `${apiUrl}/api/auth/verify`,
        { method: 'POST', body: { address, message, signature } },
      )

      authStore.setAuth(address, token, business_id)
      toast.add({
        title: 'Connected',
        description: `${address.slice(0, 6)}...${address.slice(-4)}`,
        color: 'success',
      })
    }
    catch (err: unknown) {
      const msg
        = (err as { data?: { error?: string } })?.data?.error
        ?? (err as { message?: string })?.message
        ?? 'Connection failed'
      const silent = ['user closed', 'cancelled', 'abort', 'rejected'].some(s =>
        msg.toLowerCase().includes(s),
      )
      if (!silent) {
        toast.add({ title: 'Failed to connect', description: msg, color: 'error' })
      }
    }
    finally {
      actionLoading.value = false
    }
  }

  const logout = async () => {
    actionLoading.value = true
    try {
      await w3aDisconnect()
    }
    catch {}
    finally {
      authStore.logout()
      actionLoading.value = false
      toast.add({ title: 'Disconnected', color: 'neutral' })
    }
  }

  const signMessage = async (message: string): Promise<string> => {
    if (!provider.value) throw new Error('Not connected')
    const accounts = (await provider.value.request({ method: 'eth_accounts' })) as string[]
    return provider.value.request({
      method: 'personal_sign',
      params: [message, accounts[0]],
    }) as Promise<string>
  }

  return {
    login,
    logout,
    signMessage,
    isLoading,
    isConnected: computed(() => authStore.isConnected),
    safeAddress: computed(() => authStore.address),
    address: computed(() => authStore.address),
  }
}
