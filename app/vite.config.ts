import {defineConfig} from 'vite'
import react from '@vitejs/plugin-react'
import {createMpaPlugin, createPages} from 'vite-plugin-virtual-mpa'

const apiTarget = process.env.VITE_API_TARGET || 'http://127.0.0.1:3001'

// https://vite.dev/config/
export default defineConfig({
    plugins: [
        react(),
        createMpaPlugin({
            pages: createPages([
                {
                    name: 'index',
                    filename: 'index.html',
                },
                {
                    name: 'auth',
                    filename: 'auth/index.html',
                }
            ]),
        })
    ],
    server: {
        proxy: {
            '/api': {
                target: apiTarget,
                changeOrigin: true,
            }
        }
    },
    preview: {
        host: true,
        allowedHosts: true,
        proxy: {
            '/api': {
                target: apiTarget,
                changeOrigin: true,
            }
        }
    }
})
