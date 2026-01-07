import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 7435,
    proxy: {
      '/api': {
        target: 'http://localhost:7434',
        changeOrigin: true
      },
      '/ws': {
        target: 'ws://localhost:7434',
        ws: true
      }
    }
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true
  }
})
