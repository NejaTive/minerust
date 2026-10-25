import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      // Результат `wasm-pack build engine --target web`
      '@engine': fileURLToPath(new URL('./engine/pkg', import.meta.url)),
    },
  },
})
