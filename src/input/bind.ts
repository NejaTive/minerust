import { classifyKeyDown, GAME_KEYS } from './keys'
import { stepSlot, WheelAccumulator } from './wheel'

/** Что вводу нужно знать об игре и куда он сообщает; ввод не знает ни Vue, ни wasm. */
export interface InputTarget {
  /** Движок, если он уже создан. */
  engine(): InputEngine | undefined
  canvas(): HTMLElement | undefined
  showDebug(): boolean
  slot(): number
  slotCount: number
  selectSlot(slot: number): void
  changeRenderDistance(delta: number): void
  /** Нажата N. */
  askNewWorld(): void
  /** Курсор захвачен или отпущен (после этого движок уже поставлен на паузу или снят). */
  lockChanged(locked: boolean): void
}

/** Часть `Engine`, нужная вводу. */
export interface InputEngine {
  mouse_move(dx: number, dy: number): void
  mouse_button(button: number, pressed: boolean): void
  set_key(code: string, pressed: boolean): void
  set_time_fast(fast: boolean): void
  toggle_noclip(): void
  release_all_keys(): void
  set_paused(paused: boolean): void
}

/** Подписывается на события браузера; возвращает функцию отписки и признак захвата курсора. */
export function bindInput(target: InputTarget): { unbind(): void; isLocked(): boolean } {
  let locked = false
  const wheel = new WheelAccumulator()

  // Пауза следует за курсором: Esc браузер забирает себе и до страницы не доводит, так что
  // единственный надёжный сигнал выхода — это событие.
  const onLockChange = () => {
    locked = document.pointerLockElement === target.canvas()
    const engine = target.engine()
    engine?.set_paused(!locked)
    if (!locked) engine?.release_all_keys()
    target.lockChanged(locked)
  }
  const onMouseMove = (e: MouseEvent) => {
    if (locked) target.engine()?.mouse_move(e.movementX, e.movementY)
  }
  // Только при уже захваченном курсоре: клик, который захватывает курсор, блок не ломает.
  const onMouseDown = (e: MouseEvent) => {
    if (locked) target.engine()?.mouse_button(e.button, true)
  }
  // Отпускание передаём всегда: курсор могли отпустить посреди удержания.
  const onMouseUp = (e: MouseEvent) => target.engine()?.mouse_button(e.button, false)
  const onWheel = (e: WheelEvent) => {
    if (!locked) return
    // Колесо при захваченном курсоре не должно масштабировать или листать страницу.
    e.preventDefault()
    const step = wheel.push(e.deltaY, e.deltaMode)
    if (step !== 0) target.selectSlot(stepSlot(target.slot(), step, target.slotCount))
  }
  const onKeyDown = (e: KeyboardEvent) => {
    if (locked && GAME_KEYS.has(e.code)) e.preventDefault()
    const command = classifyKeyDown(e, locked, target.showDebug())
    switch (command.type) {
      case 'distance':
        return target.changeRenderDistance(command.delta)
      case 'new-world':
        return target.askNewWorld()
      case 'noclip':
        return target.engine()?.toggle_noclip()
      case 'time-fast':
        return target.engine()?.set_time_fast(true)
      case 'slot':
        return target.selectSlot(command.slot)
      case 'key':
        return target.engine()?.set_key(e.code, true)
    }
  }
  const onKeyUp = (e: KeyboardEvent) => {
    const engine = target.engine()
    if (e.code === 'KeyT') engine?.set_time_fast(false)
    engine?.set_key(e.code, false)
  }
  const onBlur = () => {
    const engine = target.engine()
    engine?.set_time_fast(false)
    engine?.release_all_keys()
  }

  document.addEventListener('pointerlockchange', onLockChange)
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mousedown', onMouseDown)
  document.addEventListener('mouseup', onMouseUp)
  document.addEventListener('wheel', onWheel, { passive: false })
  window.addEventListener('keydown', onKeyDown)
  window.addEventListener('keyup', onKeyUp)
  window.addEventListener('blur', onBlur)

  return {
    isLocked: () => locked,
    unbind() {
      document.removeEventListener('pointerlockchange', onLockChange)
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mousedown', onMouseDown)
      document.removeEventListener('mouseup', onMouseUp)
      document.removeEventListener('wheel', onWheel)
      window.removeEventListener('keydown', onKeyDown)
      window.removeEventListener('keyup', onKeyUp)
      window.removeEventListener('blur', onBlur)
    },
  }
}
