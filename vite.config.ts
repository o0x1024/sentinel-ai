import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from 'node:url';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [vue()],

  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@components': fileURLToPath(new URL('./src/components', import.meta.url)),
      '@views': fileURLToPath(new URL('./src/views', import.meta.url)),
      '@assets': fileURLToPath(new URL('./src/assets', import.meta.url))
    }
  },

  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'vue-vendor': ['vue', 'vue-router', 'pinia'],
          'ui-vendor': ['@headlessui/vue', '@heroicons/vue'], 
          'chart-vendor': ['chart.js', 'vue-chartjs'],
          'utils-vendor': ['@vueuse/core', 'date-fns', 'axios', 'uuid']
        }
      }
    },
    minify: 'terser',
    terserOptions: {
      compress: {
        drop_console: true,
        drop_debugger: true
      }
    },
    sourcemap: false,
    target: 'esnext'
  },

  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? {
      protocol: "ws",
      host,
      port: 1421,
    } : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    }
  },

  clearScreen: false,

  optimizeDeps: {
    include: ['vue', 'vue-router', 'pinia'],
    exclude: ['@tauri-apps/api']
  }
}); 