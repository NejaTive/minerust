import { execSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

const { version } = JSON.parse(readFileSync('./package.json', 'utf8'))

// Без git (архив исходников) хеша нет — версия всё равно показывается.
function commit(): string {
  try {
    return execSync('git rev-parse --short HEAD', { stdio: ['ignore', 'pipe', 'ignore'] })
      .toString()
      .trim()
  } catch {
    return ''
  }
}

export default defineConfig({
  plugins: [vue()],
  define: {
    __APP_VERSION__: JSON.stringify(version),
    __APP_COMMIT__: JSON.stringify(commit()),
  },
  // ES-воркеры: у воркера есть динамический import wasm, формат по умолчанию (iife) его не соберёт.
  worker: { format: 'es' },
  resolve: {
    alias: {
      // Результат `wasm-pack build engine/crates/web --target web --out-dir ../../pkg`
      '@engine': fileURLToPath(new URL('./engine/pkg', import.meta.url)),
      // Результат `wasm-pack build engine/crates/worker --target web --out-dir ../../pkg-worker`
      '@engine-worker': fileURLToPath(new URL('./engine/pkg-worker', import.meta.url)),
    },
  },
})
