export const CONFIG = {
  APP_NAME: 'GasRelayer',
  LOGO_ICON: 'i-heroicons-fire-20-solid',

  NETWORKS: {
    1: { name: 'Ethereum', symbol: 'ETH', color: 'sky' },
    11155111: { name: 'Sepolia', symbol: 'ETH', color: 'violet' },
    137: { name: 'Polygon', symbol: 'MATIC', color: 'purple' },
    8453: { name: 'Base', symbol: 'ETH', color: 'blue' },
    42161: { name: 'Arbitrum', symbol: 'ETH', color: 'sky' },
    10: { name: 'Optimism', symbol: 'ETH', color: 'red' },
  } as Record<number, { name: string; symbol: string; color: string }>,

  networkName: (chainId: number): string =>
    CONFIG.NETWORKS[chainId]?.name ?? `Chain ${chainId}`,

  networkSymbol: (chainId: number): string =>
    CONFIG.NETWORKS[chainId]?.symbol ?? 'ETH',

  truncateAddress: (addr: string, start = 6, end = 4): string =>
    addr ? `${addr.slice(0, start)}...${addr.slice(-end)}` : '',
}
