import { createApp } from 'vue'
import App from './App.vue'
import { loadEngineInfo } from './engine/info'
import './style.css'

// Настройки зависят от пределов и значений по умолчанию из движка, поэтому wasm
// загружается до монтирования.
loadEngineInfo().then(
  () => createApp(App).mount('#app'),
  (e: unknown) => {
    document.getElementById('app')!.textContent = `Не удалось загрузить движок: ${String(e)}`
  },
)
