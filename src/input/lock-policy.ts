export const INVENTORY_LOCK_GRACE_MS = 400

export type PauseEscAction = 'none' | 'back' | 'resume'

export function pauseMenuEscAction(opts: {
  inWorld: boolean
  started: boolean
  busy: boolean
  hasError: boolean
  bench: boolean
  onMainScreen: boolean
}): PauseEscAction {
  if (opts.bench || opts.hasError) return 'none'
  if (!opts.onMainScreen) return 'back'
  if (!opts.inWorld || !opts.started || opts.busy) return 'none'
  return 'resume'
}

export type StartOverlay = 'none' | 'loading' | 'click'

export function startOverlay(opts: {
  inWorld: boolean
  pendingStart: boolean
  busy: boolean
  locked: boolean
  hasError: boolean
}): StartOverlay {
  if (!opts.inWorld || !opts.pendingStart || opts.locked || opts.hasError) return 'none'
  return opts.busy ? 'loading' : 'click'
}
