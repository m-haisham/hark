import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      '/sse': 'http://localhost:3000',
      '/connections': 'http://localhost:3000',
      '/test-connection': 'http://localhost:3000',
      '/health-check': 'http://localhost:3000',
    },
  },
})
