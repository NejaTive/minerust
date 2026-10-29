export const INVENTORY_LOCK_GRACE_MS = 400

export type PauseEscAction = 'none' | 'back-to-main' | 'resume'

export function pauseMenuEscAction(opts: {
  started: boolean
  busy: boolean
  hasError: boolean
  bench: boolean
  onMainScreen: boolean
}): PauseEscAction {
  if (opts.bench || opts.hasError) return 'none'
  if (!opts.onMainScreen) return 'back-to-main'
  if (!opts.started || opts.busy) return 'none'
  return 'resume'
}
