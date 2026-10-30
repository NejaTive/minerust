import type { WorldMeta } from './storage'

export type MetaStatus = 'ok' | 'newer' | 'broken'

export function isReadable(meta: WorldMeta): boolean {
  return (
    Number.isInteger(meta.version) &&
    meta.version >= 1 &&
    Number.isInteger(meta.seed) &&
    Array.isArray(meta.player) &&
    meta.player.length === 6 &&
    meta.player.every(Number.isFinite)
  )
}

export function checkMeta(meta: WorldMeta, formatVersion: number): MetaStatus {
  if (!isReadable(meta)) return 'broken'
  return meta.version > formatVersion ? 'newer' : 'ok'
}
