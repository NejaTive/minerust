import { classifyKeyDown, GAME_KEYS } from './keys'
import { armMouseGuard, filterMouseDelta, type MouseGuard } from './mouse-guard'
import { releasePointerLock, requestPointerLock } from './pointer-lock'
import { stepSlot, WheelAccumulator } from './wheel'

export interface InputTarget {
  engine(): InputEngine | undefined
  canvas(): HTMLElement | undefined
  showDebug(): boolean
  slot(): number
  slotCount: number
  selectSlot(slot: number): void
  changeRenderDistance(delta: number): void
  lockChanged(locked: boolean): void
  lockLostAfterInventory(): void
  inventoryGrace(): boolean
  inventoryOpen(): boolean
  openInventory(): void
  closeInventory(): void
}

export interface InputEngine {
  mouse_move(dx: number, dy: number): void
  mouse_button(button: number, pressed: boolean): void
  set_key(code: string, pressed: boolean): void
  set_time_fast(fast: boolean): void
  cycle_weather(): void
  toggle_noclip(): void
  release_all_keys(): void
  set_paused(paused: boolean): void
}

export function bindInput(target: InputTarget): { unbind(): void; isLocked(): boolean } {
  let locked = false
  let mouseGuard: MouseGuard | null = null
  const wheel = new WheelAccumulator()

  const onLockChange = () => {
    locked = document.pointerLockElement === target.canvas()
    const engine = target.engine()
    if (locked) {
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
  const onMouseDown = (e: MouseEvent) => {
    if (locked) target.engine()?.mouse_button(e.button, true)
  }
  const onMouseUp = (e: MouseEvent) => target.engine()?.mouse_button(e.button, false)
  const onWheel = (e: WheelEvent) => {
    if (!locked) return
    e.preventDefault()
    const step = wheel.push(e.deltaY, e.deltaMode)
    if (step !== 0) target.selectSlot(stepSlot(target.slot(), step, target.slotCount))
  }
  const onKeyDown = (e: KeyboardEvent) => {
    const inv = target.inventoryOpen()
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
      e.preventDefault()
      e.stopImmediatePropagation()
      return
    }

    if (inv) {
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
        default:
          return
      }
    }

    if (locked && GAME_KEYS.has(e.code)) e.preventDefault()
    const command = classifyKeyDown(e, locked, target.showDebug())
    switch (command.type) {
      case 'distance':
        return target.changeRenderDistance(command.delta)
      case 'noclip':
        return target.engine()?.toggle_noclip()
      case 'time-fast':
        return target.engine()?.set_time_fast(true)
      case 'weather':
        return target.engine()?.cycle_weather()
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

export function beginInventory(engine: InputEngine | undefined) {
  engine?.release_all_keys()
  releasePointerLock()
}

export async function endInventory(canvas: HTMLElement | undefined): Promise<boolean> {
  return requestPointerLock(canvas)
}
