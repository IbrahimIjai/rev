import { useAuthStore } from '~/stores/useAuthStore'

// ─── Response types matching backend JSON ────────────────────────────────────

export interface ProjectSummary {
  id: string
  name: string
  chainId: number
  forwarderAddress: string
  active: boolean
  createdAt: string
}

export interface SpendingLimits {
  dailyGasQuotaPerUser: string | null
  maxGasPerRequest: string
  maxGasPriceGwei: number
  rateLimitPerMinute: number
  allowedTargets: { type: string, addresses?: string[] }
  allowedSelectors: string[]
  webhookUrl: string | null
}

export interface ProjectDetail extends ProjectSummary {
  gasTankAddress: string | null
  relayerAddress: string | null
  spendingLimits: SpendingLimits | null
}

export interface CreateProjectResponse {
  projectId: string
  gasTankAddress: string
  relayerAddress: string
  apiKey: string
}

export interface ApiKeyCreated {
  id: string
  apiKey: string
  name: string
  createdAt: string
}

// ─── Composable ──────────────────────────────────────────────────────────────

export const useApi = () => {
  const authStore = useAuthStore()
  const config = useRuntimeConfig()

  const base = config.public.apiUrl as string

  const headers = (): Record<string, string> =>
    authStore.token ? { Authorization: `Bearer ${authStore.token}` } : {}

  const get = <T>(path: string) =>
    $fetch<T>(`${base}${path}`, { headers: headers() })

  const post = <T>(path: string, body: Record<string, unknown>) =>
    $fetch<T>(`${base}${path}`, { method: 'POST', body, headers: headers() })

  const put = <T>(path: string, body: Record<string, unknown>) =>
    $fetch<T>(`${base}${path}`, { method: 'PUT', body, headers: headers() })

  return {
    projects: {
      list: () => get<ProjectSummary[]>('/api/projects'),
      get: (id: string) => get<ProjectDetail>(`/api/projects/${id}`),
      create: (body: { name: string, chainId: number, forwarderAddress: string }) =>
        post<CreateProjectResponse>('/api/projects', body),
      updateLimits: (id: string, body: Partial<SpendingLimits & { maxGasPriceGwei: number, rateLimitPerMinute: number, webhookUrl: string }>) =>
        put<{ status: string }>(`/api/projects/${id}/limits`, body),
      createApiKey: (id: string, name: string) =>
        post<ApiKeyCreated>(`/api/projects/${id}/api-keys`, { name })
    }
  }
}
