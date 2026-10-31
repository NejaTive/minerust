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
  | { type: 'noclip' }
  | { type: 'time-fast' }
  | { type: 'weather' }
  | { type: 'slot'; slot: number }
  | { type: 'key' }

const HOTKEYS = new Set(['BracketLeft', 'BracketRight'])
const NONE: KeyCommand = { type: 'none' }

const hasModifier = (e: KeyInfo) => e.ctrlKey || e.metaKey || e.altKey

export function classifyKeyDown(e: KeyInfo, locked: boolean, showDebug: boolean): KeyCommand {
  if (HOTKEYS.has(e.code) && !hasModifier(e)) {
    return { type: 'distance', delta: e.code === 'BracketLeft' ? -1 : 1 }
  }
  if (!locked || e.repeat) return NONE
  if (e.code === 'KeyV' && showDebug && !hasModifier(e)) return { type: 'noclip' }
  if (e.code === 'KeyT' && showDebug && !hasModifier(e)) return { type: 'time-fast' }
  if (e.code === 'KeyR' && showDebug && !hasModifier(e)) return { type: 'weather' }
  const digit = /^Digit([1-9])$/.exec(e.code)
  if (digit) return hasModifier(e) ? NONE : { type: 'slot', slot: Number(digit[1]) - 1 }
  return { type: 'key' }
}
