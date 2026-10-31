export const GAME_KEYS = new Set(['Space', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'])

export interface KeyInfo {
  code: string
  repeat: boolean
  ctrlKey: boolean
  metaKey: boolean
  altKey: boolean
}

export type KeyCommand =
  | { type: 'none' }
  | { type: 'distance'; delta: -1 | 1 }
  | { type: 'new-world' }
  | { type: 'noclip' }
  | { type: 'time-fast' }
  | { type: 'weather' }
  | { type: 'slot'; slot: number }
  | { type: 'key' }

const HOTKEYS = new Set(['KeyN', 'BracketLeft', 'BracketRight'])
const NONE: KeyCommand = { type: 'none' }

const hasModifier = (e: KeyInfo) => e.ctrlKey || e.metaKey || e.altKey

export function classifyKeyDown(e: KeyInfo, locked: boolean, showDebug: boolean): KeyCommand {
  if (HOTKEYS.has(e.code) && !hasModifier(e)) {
    if (e.code === 'BracketLeft') return { type: 'distance', delta: -1 }
    if (e.code === 'BracketRight') return { type: 'distance', delta: 1 }
    return e.repeat ? NONE : { type: 'new-world' }
  }
  if (!locked || e.repeat) return NONE
  if (e.code === 'KeyV' && showDebug && !hasModifier(e)) return { type: 'noclip' }
  if (e.code === 'KeyT' && showDebug && !hasModifier(e)) return { type: 'time-fast' }
  if (e.code === 'KeyR' && showDebug && !hasModifier(e)) return { type: 'weather' }
  const digit = /^Digit([1-9])$/.exec(e.code)
  if (digit) return hasModifier(e) ? NONE : { type: 'slot', slot: Number(digit[1]) - 1 }
  return { type: 'key' }
}
