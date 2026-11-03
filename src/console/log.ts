export interface LogEntry {
  id: number
  text: string
  ok: boolean
  at: number
  suggest?: string
}

export const LOG_TTL_MS = 8000
export const LOG_VISIBLE = 6
export const LOG_VISIBLE_OPEN = 12
export const LOG_KEEP = 60

export function appendLog(
  entries: readonly LogEntry[],
  text: string,
  ok: boolean,
  now: number,
  firstId: number,
  suggest?: string,
): LogEntry[] {
  const added = text.split('\n').map((line, i) => ({
    id: firstId + i,
    text: line,
    ok,
    at: now,
    suggest,
  }))
  return [...entries, ...added].slice(-LOG_KEEP)
}

export function visibleLog(
  entries: readonly LogEntry[],
  now: number,
  consoleOpen: boolean,
): LogEntry[] {
  if (consoleOpen) return entries.slice(-LOG_VISIBLE_OPEN)
  return entries.filter((e) => now - e.at < LOG_TTL_MS).slice(-LOG_VISIBLE)
}
