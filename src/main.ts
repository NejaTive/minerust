import { createApp } from 'vue'
import App from './App.vue'
import { loadEngineInfo } from './engine/info'
import { texturePacks } from './engine/useTexturePack'
import { bindPackGui } from './ui/packGui'
import './ui/pixel.css'

bindPackGui(texturePacks.state, document.documentElement)

loadEngineInfo().then(
  () => {
    void texturePacks.restore()
    createApp(App).mount('#app')
  },
  (e: unknown) => {
    document.getElementById('app')!.textContent = `Не удалось загрузить движок: ${String(e)}`
  },
)
