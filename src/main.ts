import { createApp } from 'vue'
import App from './App.vue'
import { loadEngineInfo } from './engine/info'
import './ui/pixel.css'

loadEngineInfo().then(
  () => createApp(App).mount('#app'),
  (e: unknown) => {
    document.getElementById('app')!.textContent = `Не удалось загрузить движок: ${String(e)}`
  },
)
