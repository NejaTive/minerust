import type { WorldMeta } from './storage'

/** `ok` — грузим; `newer` — записан более новой игрой, не трогаем; `broken` — не читается. */
export type MetaStatus = 'ok' | 'newer' | 'broken'

/**
 * Метаданные читаются: версия формата не раньше первой, seed и позиция на месте.
 * Колонки старых версий движок мигрирует при импорте, поэтому старше — это `ok`.
 */
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
