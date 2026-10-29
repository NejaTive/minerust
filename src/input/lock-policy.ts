/**
 * Окно после успешного закрытия инвентаря: Chrome может выдать lock из keydown/keyup Esc
 * и тут же снять его тем же жестом (обработка Escape / автоповтор). 400 мс — больше
 * одного кадра и типичной задержки pointerlockchange, меньше сознательного «выйти в меню».
 */
export const INVENTORY_LOCK_GRACE_MS = 400

export type PauseEscAction = 'none' | 'back-to-main' | 'resume'

/**
 * Esc в меню паузы: из игры меню открывает браузер (снятие lock) — сюда не попадает.
 * Инвентарь обрабатывает Esc сам; PauseMenu при открытом инвентаре не смонтирован.
 */
export function pauseMenuEscAction(opts: {
  started: boolean
  busy: boolean
  hasError: boolean
  bench: boolean
  /** Главный экран меню (не settings/controls/confirm). */
  onMainScreen: boolean
}): PauseEscAction {
  if (opts.bench || opts.hasError) return 'none'
  // Подменю — назад даже при busy (например confirm на «Новый мир»).
  if (!opts.onMainScreen) return 'back-to-main'
  // До первого «Играть» / во время загрузки Esc на титуле ничего не делает.
  if (!opts.started || opts.busy) return 'none'
  return 'resume'
}
