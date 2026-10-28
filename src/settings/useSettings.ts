import { computed, reactive, ref, watch } from 'vue'
import { BENCH, BENCH_DEBUG, BENCH_RENDER_DISTANCE } from '../bench/config'
import { engineInfo } from '../engine/info'
import { HOTBAR, loadHotbarSlot, saveHotbarSlot } from './hotbar'
import { defaultSettings, loadSettings, saveSettings, type Settings } from './settings'
import { localStore } from './store'

let shared: ReturnType<typeof create> | undefined

function create() {
  const { limits, defaults } = engineInfo()
  const settings = reactive<Settings>(loadSettings(localStore, limits, defaultSettings(defaults)))
  // Режим замера: фиксированные настройки, поверх сохранённых, и в localStorage они не попадают.
  if (BENCH) {
    Object.assign(settings, {
      renderDistance: BENCH_RENDER_DISTANCE,
      ambientOcclusion: true,
      showDebug: BENCH_DEBUG,
    })
  }
  watch(settings, (value) => {
    if (!BENCH) saveSettings(localStore, value)
  })

  const slot = ref(loadHotbarSlot(localStore))
  watch(slot, (value) => saveHotbarSlot(localStore, value))
  // Блок в выбранном слоте; `null` — слот пуст.
  const block = computed(() => HOTBAR[slot.value] ?? null)

  return {
    settings,
    limits,
    slot,
    block,
    update(patch: Partial<Settings>) {
      Object.assign(settings, patch)
    },
  }
}

/** Настройки игры и выбранный слот хотбара; общие на приложение. Нужен загруженный `engineInfo`. */
export function useSettings() {
  shared ??= create()
  return shared
}
