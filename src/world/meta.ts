import { GAME_MODES, WORLD_TYPES } from './kinds'
import type { WorldMeta } from './storage'

export type MetaStatus = 'ok' | 'newer' | 'broken'

const isOneOf = (values: readonly string[], value: unknown) =>
  typeof value === 'string' && values.includes(value)

const isTimestamp = (value: unknown) => typeof value === 'number' && Number.isFinite(value)

export function isReadable(meta: WorldMeta): boolean {
  return (
    Number.isInteger(meta.version) &&
    meta.version >= 1 &&
    Number.isInteger(meta.seed) &&
    typeof meta.name === 'string' &&
    isOneOf(GAME_MODES, meta.mode) &&
    isOneOf(WORLD_TYPES, meta.worldType) &&
    isTimestamp(meta.createdAt) &&
    isTimestamp(meta.lastPlayed) &&
    (meta.player === undefined ||
      (Array.isArray(meta.player) &&
        meta.player.length === 6 &&
        meta.player.every(Number.isFinite)))
  )
}

export function checkMeta(meta: WorldMeta, formatVersion: number): MetaStatus {
  if (!isReadable(meta)) return 'broken'
  return meta.version > formatVersion ? 'newer' : 'ok'
}
