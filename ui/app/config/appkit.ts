import { WagmiAdapter } from '@reown/appkit-adapter-wagmi'
import { baseSepolia } from '@reown/appkit/networks'

export const projectId = '43f98c33e7c39797ecd4970c9781666f'

export const networks = [baseSepolia] as [typeof baseSepolia]

export const wagmiAdapter = new WagmiAdapter({
  networks,
  projectId
})
