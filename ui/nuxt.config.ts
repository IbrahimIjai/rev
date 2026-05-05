export default defineNuxtConfig({
  ssr: false,

  modules: [
    '@nuxt/eslint',
    '@nuxt/ui',
    '@pinia/nuxt',
    '@wagmi/vue/nuxt',
  ],

  devtools: { enabled: true },

  css: ['~/assets/css/main.css'],

  future: { compatibilityVersion: 4 },

  compatibilityDate: '2025-01-15',

  eslint: {
    config: {
      stylistic: { commaDangle: 'never', braceStyle: '1tbs' },
    },
  },

  colorMode: {
    preference: 'system',
    fallback: 'dark',
    classSuffix: '',
  },

  runtimeConfig: {
    public: {
       projectId: process.env.NUXT_PROJECT_ID,
      network: process.env.NUXT_PUBLIC_NETWORK || 'mainnet',
      apiUrl: process.env.NUXT_PUBLIC_API_URL || 'http://localhost:8080',
    },
  },

  vite: {
    define: {
      global: 'globalThis',
    },
    optimizeDeps: {
      include: ['@reown/appkit', '@reown/appkit-adapter-wagmi'],
    },
  },
})
