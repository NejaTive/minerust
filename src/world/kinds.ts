export const GAME_MODES = ['creative'] as const
export const WORLD_TYPES = ['standard', 'flat', 'debug', 'structures'] as const

export type GameMode = (typeof GAME_MODES)[number]
export type WorldType = (typeof WORLD_TYPES)[number]

export const MODE_LABELS: Record<GameMode, string> = { creative: 'Творческий' }
export const WORLD_TYPE_LABELS: Record<WorldType, string> = {
  standard: 'Стандартный',
  flat: 'Плоский',
  debug: 'Отладка',
  structures: 'Структуры',
}

const DEBUG_ONLY_TYPES: readonly WorldType[] = ['debug', 'structures']

export function selectableWorldTypes(showDebug: boolean): readonly WorldType[] {
  return showDebug ? WORLD_TYPES : WORLD_TYPES.filter((type) => !DEBUG_ONLY_TYPES.includes(type))
}

export function cycle<T>(values: readonly T[], current: T): T {
  const index = values.indexOf(current)
  return values[(index + 1) % values.length]!
}
