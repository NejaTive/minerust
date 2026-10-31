export const GAME_MODES = ['creative'] as const
export const WORLD_TYPES = ['standard', 'flat'] as const

export type GameMode = (typeof GAME_MODES)[number]
export type WorldType = (typeof WORLD_TYPES)[number]

export const MODE_LABELS: Record<GameMode, string> = { creative: 'Творческий' }
export const WORLD_TYPE_LABELS: Record<WorldType, string> = {
  standard: 'Стандартный',
  flat: 'Плоский',
}

export function cycle<T>(values: readonly T[], current: T): T {
  const index = values.indexOf(current)
  return values[(index + 1) % values.length]!
}
