export interface BenchParams {
  enabled: boolean
  debug: boolean
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
