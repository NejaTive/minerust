import type { KeyValueStore } from './store'

const HOTBAR_SLOT_KEY = 'minerust.hotbarSlot'
const HOTBAR_CONTENTS_KEY = 'minerust.hotbar'

export const HOTBAR_SIZE = 9

export const DEFAULT_HOTBAR: (string | null)[] = Array.from({ length: HOTBAR_SIZE }, () => null)

export type HotbarContents = (string | null)[]

export function loadHotbarSlot(store: KeyValueStore, count = HOTBAR_SIZE): number {
  const raw = store.get(HOTBAR_SLOT_KEY)
  const value = raw === null ? Number.NaN : Number(raw)
  return Number.isInteger(value) && value >= 0 && value < count ? value : 0
}

export function saveHotbarSlot(store: KeyValueStore, slot: number) {
  store.set(HOTBAR_SLOT_KEY, String(slot))
}

export function loadHotbarContents(
  store: KeyValueStore,
  known: ReadonlySet<string>,
): HotbarContents {
  const raw = store.get(HOTBAR_CONTENTS_KEY)
  if (raw === null) return [...DEFAULT_HOTBAR]
  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch {
    return [...DEFAULT_HOTBAR]
  }
  if (!Array.isArray(parsed) || parsed.length !== HOTBAR_SIZE) return [...DEFAULT_HOTBAR]
  const slots: HotbarContents = []
  for (const item of parsed) {
    if (item === null) {
      slots.push(null)
    } else if (typeof item === 'string' && known.has(item)) {
      slots.push(item)
    } else {
      return [...DEFAULT_HOTBAR]
    }
  }
  return slots
}

export function saveHotbarContents(store: KeyValueStore, slots: HotbarContents) {
  store.set(HOTBAR_CONTENTS_KEY, JSON.stringify(slots))
}

export function clearHotbar(slots: HotbarContents) {
  for (let i = 0; i < slots.length; i++) slots[i] = null
}

export function putHotbarSlot(slots: HotbarContents, index: number, name: string | null) {
  if (index < 0 || index >= slots.length) return
  slots[index] = name
}

export function swapHotbarSlots(slots: HotbarContents, a: number, b: number) {
  if (a < 0 || b < 0 || a >= slots.length || b >= slots.length || a === b) return
  const tmp = slots[a]
  slots[a] = slots[b]
  slots[b] = tmp
}

export function clearHotbarSlot(slots: HotbarContents, index: number) {
  putHotbarSlot(slots, index, null)
}
