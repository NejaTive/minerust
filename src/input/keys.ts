/** Клавиши, для которых гасим действие браузера по умолчанию (скролл страницы и т.п.). */
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
  | { type: 'slot'; slot: number }
  /** Клавиша уходит в движок как есть. */
  | { type: 'key' }

const HOTKEYS = new Set(['KeyN', 'BracketLeft', 'BracketRight'])
const NONE: KeyCommand = { type: 'none' }

const hasModifier = (e: KeyInfo) => e.ctrlKey || e.metaKey || e.altKey

/**
 * Что делать с нажатием. Хоткеи интерфейса работают без pointer lock и в движок не попадают;
 * остальное — только при захваченном курсоре. Отладочные V и T — только при `showDebug`.
 */
export function classifyKeyDown(e: KeyInfo, locked: boolean, showDebug: boolean): KeyCommand {
  // Cmd+H / Ctrl+G и т.п. — системные сочетания, их не перехватываем.
  if (HOTKEYS.has(e.code) && !hasModifier(e)) {
    // Дальность при удержании листается автоповтором; N — по одному нажатию.
    if (e.code === 'BracketLeft') return { type: 'distance', delta: -1 }
    if (e.code === 'BracketRight') return { type: 'distance', delta: 1 }
    return e.repeat ? NONE : { type: 'new-world' }
  }
  if (!locked || e.repeat) return NONE
  // Полёт сквозь блоки и перемотка времени (×60, пока T зажата) — отладочные.
  if (e.code === 'KeyV' && showDebug && !hasModifier(e)) return { type: 'noclip' }
  if (e.code === 'KeyT' && showDebug && !hasModifier(e)) return { type: 'time-fast' }
  // Цифры выбирают слот и в движок как клавиши не попадают.
  const digit = /^Digit([1-9])$/.exec(e.code)
  if (digit) return hasModifier(e) ? NONE : { type: 'slot', slot: Number(digit[1]) - 1 }
  return { type: 'key' }
}
