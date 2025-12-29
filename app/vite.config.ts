import {defineConfig} from 'vite'
import react from '@vitejs/plugin-react'
import {createMpaPlugin, createPages} from 'vite-plugin-virtual-mpa'


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
                target: 'http://localhost:3000',
                changeOrigin: true,
            }
        }
    },
    preview: {
        host: true,
        allowedHosts: true,
        proxy: {
            '/api': {
                target: 'http://localhost:3000',
                changeOrigin: true,
            }
        }
    }
})
