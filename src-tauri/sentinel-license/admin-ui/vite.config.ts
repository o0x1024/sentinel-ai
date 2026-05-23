import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'node:path'

export default defineConfig({
  plugins: [vue()],
  root: __dirname,
  base: '/admin/',
  build: {
    outDir: resolve(__dirname, '../src/bin/entitlement_server/admin_ui_vue_dist'),
    emptyOutDir: true,
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        entryFileNames: 'assets/admin.js',
        chunkFileNames: 'assets/admin-[name].js',
        assetFileNames: 'assets/admin.[ext]',
      },
    },
  },
})
