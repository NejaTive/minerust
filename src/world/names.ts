export const DEFAULT_WORLD_NAME = 'Новый мир'
export const MAX_WORLD_NAME = 32

export function uniqueName(requested: string, taken: readonly string[]): string {
  const base = requested.trim().slice(0, MAX_WORLD_NAME) || DEFAULT_WORLD_NAME
  const used = new Set(taken)
  if (!used.has(base)) return base
  for (let n = 2; ; n++) {
    const candidate = `${base} (${n})`
    if (!used.has(candidate)) return candidate
  }
}
