export const HOTBAR_SIZE = 9

export const DEFAULT_HOTBAR: (string | null)[] = Array.from({ length: HOTBAR_SIZE }, () => null)

export type HotbarContents = (string | null)[]

export function parseHotbarSlot(raw: unknown, count = HOTBAR_SIZE): number {
  return typeof raw === 'number' && Number.isInteger(raw) && raw >= 0 && raw < count ? raw : 0
}

export function parseHotbarContents(raw: unknown, known: ReadonlySet<string>): HotbarContents {
  if (!Array.isArray(raw) || raw.length !== HOTBAR_SIZE) return [...DEFAULT_HOTBAR]
  const slots: HotbarContents = []
  for (const item of raw) {
    if (item === null || (typeof item === 'string' && known.has(item))) slots.push(item)
    else return [...DEFAULT_HOTBAR]
  }
  return slots
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
