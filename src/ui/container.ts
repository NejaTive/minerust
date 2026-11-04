export const CHEST_COLUMNS = 9
export const SINGLE_CHEST_SLOTS = 27

export type Slot = string | null
export type Area = 'chest' | 'hotbar'

export interface ContainerState {
  chest: Slot[]
  hotbar: Slot[]
  cursor: Slot
  origin?: { area: Area; index: number }
}

export interface ContainerLayout {
  rows: number
  title: string
  topHeight: number
  height: number
  inventoryLabelY: number
  hotbarY: number
}

const BOTTOM_HEIGHT = 96
const ROW = 18

export function containerLayout(slotCount: number): ContainerLayout {
  const rows = Math.max(1, Math.ceil(slotCount / CHEST_COLUMNS))
  const top = rows * ROW + 17
  return {
    rows,
    title: rows > 3 ? 'Большой сундук' : 'Сундук',
    topHeight: top,
    height: top + BOTTOM_HEIGHT,
    inventoryLabelY: top + 3,
    hotbarY: rows * ROW + 89,
  }
}

const slots = (state: ContainerState, area: Area) => (area === 'chest' ? state.chest : state.hotbar)

export function clickSlot(state: ContainerState, area: Area, index: number) {
  const list = slots(state, area)
  if (index < 0 || index >= list.length) return
  const held = state.cursor
  state.cursor = list[index] ?? null
  list[index] = held
  state.origin = state.cursor ? { area, index } : undefined
}

/** Предмет из руки возвращается туда, откуда его взяли, иначе в первый свободный слот. */
export function returnCursor(state: ContainerState) {
  const item = state.cursor
  if (!item) return
  const origin = state.origin
  const target = origin ? slots(state, origin.area) : undefined
  if (origin && target && !target[origin.index]) {
    target[origin.index] = item
    state.cursor = null
  } else {
    for (const list of [state.chest, state.hotbar]) {
      const free = list.findIndex((slot) => !slot)
      if (free >= 0) {
        list[free] = item
        state.cursor = null
        break
      }
    }
  }
  if (state.cursor && origin && target) {
    state.cursor = target[origin.index] ?? null
    target[origin.index] = item
  }
  state.origin = undefined
}

let beforeClose: (() => void) | undefined

export function onContainerClose(handler: (() => void) | undefined) {
  beforeClose = handler
}

export function runContainerClose() {
  beforeClose?.()
}

export function clearSlot(state: ContainerState, area: Area, index: number) {
  const list = slots(state, area)
  if (index >= 0 && index < list.length) list[index] = null
}

export function shiftClick(state: ContainerState, area: Area, index: number): boolean {
  const from = slots(state, area)
  const to = slots(state, area === 'chest' ? 'hotbar' : 'chest')
  const item = from[index]
  if (!item) return false
  const free = to.findIndex((slot) => !slot)
  if (free < 0) return false
  to[free] = item
  from[index] = null
  return true
}

export function changedSlots(before: Slot[], after: Slot[]): number[] {
  return after.flatMap((slot, i) => ((before[i] ?? null) === slot ? [] : [i]))
}
