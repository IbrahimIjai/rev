import { defineStore } from 'pinia'

export const useAuthStore = defineStore('auth', () => {
  const address = ref<string | null>(null)
  const token = ref<string | null>(null)
  const businessId = ref<string | null>(null)

  const isConnected = computed(() => !!token.value)

  function init() {
    if (!import.meta.client) return
    address.value = localStorage.getItem('gr_address')
    token.value = localStorage.getItem('gr_token')
    businessId.value = localStorage.getItem('gr_business_id')
  }

  function setAuth(addr: string, jwt: string, bizId: string) {
    address.value = addr
    token.value = jwt
    businessId.value = bizId
    if (import.meta.client) {
      localStorage.setItem('gr_address', addr)
      localStorage.setItem('gr_token', jwt)
      localStorage.setItem('gr_business_id', bizId)
    }
  }

  function logout() {
    address.value = null
    token.value = null
    businessId.value = null
    if (import.meta.client) {
      localStorage.removeItem('gr_address')
      localStorage.removeItem('gr_token')
      localStorage.removeItem('gr_business_id')
    }
  }

  return { address, token, businessId, isConnected, init, setAuth, logout }
})
