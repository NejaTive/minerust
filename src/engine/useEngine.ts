import type { Engine } from '@engine'
import { nextTick, onBeforeUnmount, onMounted, shallowRef, watch, type Ref } from 'vue'
import { BENCH, BENCH_POINTS, BENCH_SEED, BENCH_TIME } from '../bench/config'
import { FrameRecorder } from '../bench/recorder'
import { runBench } from '../bench/run'
import { bindInput } from '../input/bind'
import { releasePointerLock, requestPointerLock } from '../input/pointer-lock'
import { HOTBAR } from '../settings/hotbar'
import { clamp, columnsWord } from '../settings/settings'
import { useSettings } from '../settings/useSettings'
import { startAutosave } from '../world/autosave'
import { WorldPersistence } from '../world/persistence'
import { openStorage } from '../world/storage'
import { applyBlock, applySettings } from './apply'
import { NoWebGpuError, hasWebGpu } from './gpu'
import { FrameLoop, createEngine } from './host'
import { engineInfo } from './info'
import { gameState as state, notify } from './state'

const randomSeed = () => crypto.getRandomValues(new Int32Array(1))[0]

/**
 * Жизненный цикл движка на canvas: загрузка мира, кадровый цикл, ввод, сохранение, «Новый мир».
 * Показания пишет в `gameState`, настройки и слот читает из `useSettings`.
 * `askNewWorld` вызывается по N: подтверждение показывает меню.
 */
export function useEngine(
  canvas: Ref<HTMLCanvasElement | undefined>,
  canvasKey: Ref<number>,
  askNewWorld: () => void,
) {
  const { settings, limits, slot } = useSettings()
  const info = engineInfo()
  const benchFrames = new FrameRecorder()
  const persistence = new WorldPersistence({
    open: openStorage,
    formatVersion: info.formatVersion,
    notify,
    setNewerSave: (newer) => (state.newerSave = newer),
    randomSeed,
  })
  // shallowRef: Vue не должен оборачивать wasm-объект в Proxy.
  const engine = shallowRef<Engine>()
  let loop: FrameLoop | undefined
  // Блокирует N, пока движок пересоздаётся; до конца загрузки true.
  let switching = true
  let disposed = false
  let stopAutosave: (() => void) | undefined

  function setSwitching(value: boolean) {
    switching = value
    state.busy = value
  }

  const save = () => persistence.save(engine.value)

  const input = bindInput({
    engine: () => engine.value,
    canvas: () => canvas.value,
    showDebug: () => settings.showDebug,
    slot: () => slot.value,
    slotCount: HOTBAR.length,
    selectSlot: (value) => (slot.value = value),
    changeRenderDistance(delta) {
      const columns = clamp(
        settings.renderDistance + delta,
        limits.renderDistanceMin,
        limits.renderDistanceMax,
      )
      if (columns === settings.renderDistance) return
      // В движок значение попадёт через watch на настройки.
      settings.renderDistance = columns
      notify(`Дальность: ${columns} ${columnsWord(columns)}`)
    },
    askNewWorld() {
      if (!switching) askNewWorld()
    },
    lockChanged(locked) {
      state.locked = locked
      if (!locked) void save()
    },
  })

  watch(
    () => ({ ...settings }),
    () => engine.value && applySettings(engine.value, settings),
  )
  watch(slot, () => engine.value && applyBlock(engine.value, HOTBAR[slot.value] ?? null))

  /** `player` — `camera_state()`; без него игрок на спауне. Правки берутся из `persistence`. */
  async function start(seed: number, player?: ArrayLike<number>, time?: number) {
    const el = canvas.value!
    const created = await createEngine(el, seed, player)
    if (disposed) {
      created.free()
      return
    }
    persistence.importInto(created)
    if (BENCH) {
      created.set_time_of_day(BENCH_TIME)
      created.set_time_frozen(true)
      window.__benchEngine = created
    } else if (time !== undefined && Number.isFinite(time)) {
      created.set_time_of_day(time)
    }
    applySettings(created, settings)
    // Меню открыто (курсор не захвачен) — новый движок тоже начинает с паузы.
    created.set_paused(BENCH ? false : !input.isLocked())
    applyBlock(created, HOTBAR[slot.value] ?? null)
    engine.value = created
    state.ready = true
    loop = new FrameLoop(created, el, {
      fps: (v) => (state.fps = v),
      stats: (v) => (state.stats = v),
      time: (v) => (state.time = v),
      player: (v) => (state.player = v),
      rendered: BENCH ? (now, ms) => benchFrames.push(now, ms) : undefined,
    })
    loop.start()
  }

  /** Освобождает движок и всё, что привязано к текущему canvas. */
  function stop() {
    loop?.stop()
    loop = undefined
    if (document.pointerLockElement === canvas.value) document.exitPointerLock()
    engine.value?.free()
    engine.value = undefined
  }

  /** Подтверждение спрашивает меню; здесь мир заменяется сразу. */
  async function newWorld() {
    if (switching || !engine.value) return
    setSwitching(true)
    try {
      stop()
      await persistence.reset()
      // Смена ключа пересоздаёт <canvas>: новый мир начинается на чистом контексте, а не на
      // переконфигурированной поверхности прежнего движка.
      canvasKey.value++
      await nextTick()
      await start(randomSeed())
      // Сразу, а не через интервал: иначе перезагрузка вернула бы мир с сидом по умолчанию.
      void save()
      notify('Новый мир')
    } catch (e) {
      state.error = String(e)
    } finally {
      setSwitching(false)
    }
  }

  onMounted(async () => {
    if (!hasWebGpu()) {
      state.noWebGpu = ''
      setSwitching(false)
      return
    }
    try {
      if (BENCH) {
        // Мир в памяти: без хранилища `save` ничего не делает, автосохранения нет.
        await start(BENCH_SEED, BENCH_POINTS[0].camera)
        void runBench(() => engine.value!, benchFrames, canvas.value!, settings)
      } else {
        const world = await persistence.load()
        await start(world.seed, world.player, world.time)
        stopAutosave = startAutosave(() => void save())
      }
    } catch (e) {
      if (e instanceof NoWebGpuError) state.noWebGpu = e.message
      else state.error = String(e)
    } finally {
      setSwitching(false)
    }
  })

  onBeforeUnmount(() => {
    disposed = true
    stopAutosave?.()
    void save()
    stop()
    input.unbind()
  })

  return {
    engine,
    /** Захватить курсор; из обработчика клика. Возвращает, удалось ли. */
    requestLock: () => requestPointerLock(canvas.value),
    releaseCursor: releasePointerLock,
    newWorld,
  }
}
