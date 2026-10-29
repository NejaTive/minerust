import { classifyKeyDown, GAME_KEYS } from './keys'
import { armMouseGuard, filterMouseDelta, type MouseGuard } from './mouse-guard'
import { releasePointerLock, requestPointerLock } from './pointer-lock'
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
  /**
   * Потеря lock в окне после закрытия инвентаря: не пауза/меню, а «нужен клик».
   * Вызывать вместо `lockChanged(false)`, паузу ввод уже не ставил.
   */
  lockLostAfterInventory(): void
  /** Идёт окно после закрытия инвентаря (`INVENTORY_LOCK_GRACE_MS`). */
  inventoryGrace(): boolean
  /** Креатив-инвентарь открыт: свободный курсор без паузы. */
  inventoryOpen(): boolean
  /** Открыть инвентарь (при захваченном курсоре). */
  openInventory(): void
  /** Закрыть инвентарь и вернуть захват курсора. */
  closeInventory(): void
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
  let mouseGuard: MouseGuard | null = null
  const wheel = new WheelAccumulator()

  // Пауза следует за курсором, кроме инвентаря и «grace» после его закрытия.
  // Esc браузер забирает себе при lock, так что выход из игры — только pointerlockchange.
  const onLockChange = () => {
    locked = document.pointerLockElement === target.canvas()
    const engine = target.engine()
    if (locked) {
      // Каждый новый захват (пауза, инвентарь, клик после lockLost) — защита от скачка камеры.
      mouseGuard = armMouseGuard(performance.now())
      engine?.set_paused(false)
      target.lockChanged(true)
      return
    }
    if (target.inventoryOpen()) {
      engine?.set_paused(false)
      target.lockChanged(false)
      return
    }
    // Chrome: lock из Esc/E мог успеть выдаться и сразу сняться — не открываем PauseMenu.
    if (target.inventoryGrace()) {
      engine?.set_paused(false)
      target.lockLostAfterInventory()
      return
    }
    engine?.set_paused(true)
    engine?.release_all_keys()
    target.lockChanged(false)
  }
  const onMouseMove = (e: MouseEvent) => {
    if (!locked) return
    const result = filterMouseDelta(mouseGuard, performance.now(), e.movementX, e.movementY)
    mouseGuard = result.guard
    if (result.delta) target.engine()?.mouse_move(result.delta.dx, result.delta.dy)
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
    const inv = target.inventoryOpen()
    // E / Esc для инвентаря — до classify: иначе E ушла бы в движок как клавиша.
    if (e.code === 'KeyE' && !e.repeat && !e.ctrlKey && !e.metaKey && !e.altKey) {
      if (inv) {
        e.preventDefault()
        target.closeInventory()
        return
      }
      if (locked) {
        e.preventDefault()
        target.openInventory()
        return
      }
    }
    if (e.code === 'Escape' && inv) {
      // Закрытие и requestPointerLock — на keyup: из keydown Escape Chrome часто
      // успевает выдать lock и тем же жестом сразу снять → PauseMenu.
      e.preventDefault()
      e.stopImmediatePropagation()
      return
    }

    if (inv) {
      // Мир не слушает WASD/Space; слоты и хоткеи UI ([,], N) — можно.
      if (GAME_KEYS.has(e.code)) e.preventDefault()
      const digit = /^Digit([1-9])$/.exec(e.code)
      if (digit && !e.ctrlKey && !e.metaKey && !e.altKey) {
        target.selectSlot(Number(digit[1]) - 1)
        return
      }
      const command = classifyKeyDown(e, false, target.showDebug())
      switch (command.type) {
        case 'distance':
          return target.changeRenderDistance(command.delta)
        case 'new-world':
          return target.askNewWorld()
        default:
          return
      }
    }

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
    if (e.code === 'Escape' && target.inventoryOpen()) {
      e.preventDefault()
      e.stopImmediatePropagation()
      target.closeInventory()
      return
    }
    const engine = target.engine()
    if (e.code === 'KeyT') engine?.set_time_fast(false)
    // Пока инвентарь открыт, нажатия в движок не уходили — и отпускания тоже глушим,
    // чтобы не снять «чужой» флаг, если что-то всё же просочилось.
    if (!target.inventoryOpen()) engine?.set_key(e.code, false)
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
  // capture: Esc инвентаря раньше, чем PauseMenu (тот слушает bubble на window).
  window.addEventListener('keydown', onKeyDown, true)
  window.addEventListener('keyup', onKeyUp, true)
  window.addEventListener('blur', onBlur)

  return {
    isLocked: () => locked,
    unbind() {
      document.removeEventListener('pointerlockchange', onLockChange)
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mousedown', onMouseDown)
      document.removeEventListener('mouseup', onMouseUp)
      document.removeEventListener('wheel', onWheel)
      window.removeEventListener('keydown', onKeyDown, true)
      window.removeEventListener('keyup', onKeyUp, true)
      window.removeEventListener('blur', onBlur)
    },
  }
}

/** Открыть инвентарь: снять lock, отпустить клавиши; паузу не ставить (см. onLockChange). */
export function beginInventory(engine: InputEngine | undefined) {
  engine?.release_all_keys()
  releasePointerLock()
}

/** Закрыть инвентарь: снова захватить курсор; вызывать, пока флаг инвентаря ещё true. */
export async function endInventory(canvas: HTMLElement | undefined): Promise<boolean> {
  return requestPointerLock(canvas)
}
