import { resolve } from 'node:path'
import { defineConfig, loadEnv } from 'vite'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig(({ mode }) => {
  const rootEnv = loadEnv(mode, resolve(process.cwd(), '..'), '')
  const frontendEnv = loadEnv(mode, process.cwd(), '')
  const env = {
    ...rootEnv,
    ...frontendEnv,
    ...process.env,
  }

  const backendPort = env.BACKEND_PORT || '3001'
  const publicPort = Number.parseInt(env.DEBUG_PORT || env.VITE_DEV_PORT || '5173', 10)
  const apiTarget = env.VITE_API_TARGET || `http://127.0.0.1:${backendPort}`

  const apiProxy = {
    '/api': {
      target: apiTarget,
      changeOrigin: true,
      ws: true,
    },
  }

  return {
    plugins: [tailwindcss()],
    build: {
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (id.includes('node_modules/three')) {
              return 'three-vendor'
            }

            if (id.includes('node_modules/skinview3d')) {
              return 'skinviewer'
            }

            if (id.includes('node_modules/react-hot-toast')) {
              return 'feedback-vendor'
            }

            if (id.includes('node_modules/react') || id.includes('node_modules/react-dom')) {
              return 'react-vendor'
            }

            return undefined
          },
        },
      },
    },
    server: {
      host: '0.0.0.0',
      port: publicPort,
      proxy: apiProxy,
    },
    preview: {
      host: '0.0.0.0',
      port: 4173,
      proxy: apiProxy,
    },
  }
})
