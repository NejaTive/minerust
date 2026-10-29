export interface BenchParams {
  /** `?bench` в адресе: режим замера. */
  enabled: boolean
  /** `&debug=0` — как обычная игра с выключенным `showDebug`: без замеров кадра и HUD (для сравнения накладных). */
  debug: boolean
  /** `&workers=0` — меши строит основной поток, как до воркеров: база для сравнения замеров. */
  workers: boolean
}

export function parseBenchParams(search: string): BenchParams {
  const params = new URLSearchParams(search)
  return {
    enabled: params.has('bench'),
    debug: params.get('debug') !== '0',
    workers: params.get('workers') !== '0',
  }
}
