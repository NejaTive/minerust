import { computed, reactive, ref, watch } from 'vue'
import { BENCH, BENCH_DEBUG, BENCH_RENDER_DISTANCE } from '../bench/config'
import { engineInfo } from '../engine/info'
import {
  clearHotbar,
  clearHotbarSlot,
  loadHotbarContents,
  loadHotbarSlot,
  putHotbarSlot,
  saveHotbarContents,
  saveHotbarSlot,
  swapHotbarSlots,
} from './hotbar'
import { defaultSettings, loadSettings, saveSettings, type Settings } from './settings'
import { localStore } from './store'

let shared: ReturnType<typeof create> | undefined

function create() {
  const { limits, defaults, blocks } = engineInfo()
  const settings = reactive<Settings>(loadSettings(localStore, limits, defaultSettings(defaults)))
  if (BENCH) {
    Object.assign(settings, {
      renderDistance: BENCH_RENDER_DISTANCE,
      ambientOcclusion: true,
      clouds: true,
      showDebug: BENCH_DEBUG,
    })
  }
  watch(settings, (value) => {
    if (!BENCH) saveSettings(localStore, value)
  })

  const slot = ref(loadHotbarSlot(localStore))
  watch(slot, (value) => saveHotbarSlot(localStore, value))

  const knownNames = new Set(blocks.map((b) => b.name))
  const slots = reactive(loadHotbarContents(localStore, knownNames))
  watch(
    slots,
    (value) => {
      if (!BENCH) saveHotbarContents(localStore, [...value])
    },
    { deep: true },
  )

  const block = computed(() => slots[slot.value] ?? null)

  return {
    settings,
    limits,
    slot,
    slots,
    block,
    update(patch: Partial<Settings>) {
      Object.assign(settings, patch)
    },
    putInSlot(index: number, name: string | null) {
      putHotbarSlot(slots, index, name)
    },
    swapSlots(a: number, b: number) {
      swapHotbarSlots(slots, a, b)
    },
    clearSlot(index: number) {
      clearHotbarSlot(slots, index)
    },
    resetHotbar() {
      clearHotbar(slots)
      slot.value = 0
    },
  }
}

export function useSettings() {
  shared ??= create()
  return shared
}
