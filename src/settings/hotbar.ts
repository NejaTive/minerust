import type { KeyValueStore } from './store'

const HOTBAR_SLOT_KEY = 'minerust.hotbarSlot'

/**
 * Содержимое хотбара по слотам; `null` — пустой слот. Имена — из `blocks()` движка (тест
 * `hotbar.test.ts` сверяет их с фикстурой). Будущий инвентарь заменит константу на состояние,
 * остальной код читает только этот массив.
 */
export const HOTBAR: (string | null)[] = [
  'grass',
  'dirt',
  'stone',
  'sand',
  'log',
  'leaves',
  'glowstone',
  'torch',
  'poppy',
]

/** Выбранный слот хотбара; вне 0..count−1 или не число — первый. */
export function loadHotbarSlot(store: KeyValueStore, count = HOTBAR.length): number {
  const raw = store.get(HOTBAR_SLOT_KEY)
  const value = raw === null ? Number.NaN : Number(raw)
  return Number.isInteger(value) && value >= 0 && value < count ? value : 0
}

export function saveHotbarSlot(store: KeyValueStore, slot: number) {
  store.set(HOTBAR_SLOT_KEY, String(slot))
}
