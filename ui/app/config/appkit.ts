import { WagmiAdapter } from '@reown/appkit-adapter-wagmi'
import { mainnet, arbitrum } from '@reown/appkit/networks'

export const projectId = process.env.NUXT_PROJECT_ID || 'YOUR_PROJECT_ID'

export const networks = [mainnet, arbitrum]

export const wagmiAdapter = new WagmiAdapter({
  networks,
  projectId
})