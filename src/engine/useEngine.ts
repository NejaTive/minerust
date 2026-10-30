import type { Engine } from '@engine'
import { nextTick, onBeforeUnmount, onMounted, shallowRef, watch, type Ref } from 'vue'
import { BENCH, BENCH_POINTS, BENCH_SEED, BENCH_TIME, BENCH_WORKERS } from '../bench/config'
import { FrameRecorder } from '../bench/recorder'
import { runBench } from '../bench/run'
import { beginInventory, bindInput, endInventory } from '../input/bind'
import { inventoryFlags, type InventoryMode } from '../input/inventory-mode'
import { INVENTORY_LOCK_GRACE_MS } from '../input/lock-policy'
import { releasePointerLock, requestPointerLock } from '../input/pointer-lock'
import { HOTBAR_SIZE } from '../settings/hotbar'
import { clamp, columnsWord } from '../settings/settings'
import { useSettings } from '../settings/useSettings'
import { startAutosave } from '../world/autosave'
import { WorldPersistence } from '../world/persistence'
import { openStorage } from '../world/storage'
import { applyBlock, applySettings } from './apply'
import { NoWebGpuError, hasWebGpu } from './gpu'
import { FrameLoop, createEngine } from './host'
import { MeshPool } from './mesh-pool'
import { engineInfo } from './info'
import { gameState as state, notify } from './state'

const randomSeed = () => crypto.getRandomValues(new Int32Array(1))[0]

export function useEngine(
  canvas: Ref<HTMLCanvasElement | undefined>,
  canvasKey: Ref<number>,
  askNewWorld: () => void,
) {
  const { settings, limits, slot, block, resetHotbar } = useSettings()
  const info = engineInfo()
  const benchFrames = new FrameRecorder()
  const persistence = new WorldPersistence({
    open: openStorage,
    formatVersion: info.formatVersion,
    notify,
    setNewerSave: (newer) => (state.newerSave = newer),
    randomSeed,
  })
  const engine = shallowRef<Engine>()
  let loop: FrameLoop | undefined
  let pool: MeshPool | undefined
  let switching = true
  let disposed = false
  let stopAutosave: (() => void) | undefined
  let mode: InventoryMode = 'idle'
  let graceTimer = 0
  const modeIs = (value: InventoryMode) => mode === value

  function setMode(next: InventoryMode) {
    mode = next
    clearTimeout(graceTimer)
    if (next === 'grace') {
      graceTimer = window.setTimeout(() => mode === 'grace' && setMode('idle'), INVENTORY_LOCK_GRACE_MS)
    }
    Object.assign(state, inventoryFlags(next))
  }

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
    slotCount: HOTBAR_SIZE,
    selectSlot: (value) => (slot.value = value),
    changeRenderDistance(delta) {
      const columns = clamp(
        settings.renderDistance + delta,
        limits.renderDistanceMin,
        limits.renderDistanceMax,
      )
      if (columns === settings.renderDistance) return
      settings.renderDistance = columns
      notify(`Дальность: ${columns} ${columnsWord(columns)}`)
    },
    askNewWorld() {
      if (mode === 'inventory' || mode === 'closing' || mode === 'lock-lost') {
        setMode('idle')
        engine.value?.release_all_keys()
        engine.value?.set_paused(true)
      }
      if (!switching) askNewWorld()
    },
    lockChanged(locked) {
      state.locked = locked
      if (locked && mode === 'lock-lost') setMode('idle')
      if (!locked) void save()
    },
    lockLostAfterInventory() {
      state.locked = false
      setMode('lock-lost')
      void save()
    },
    inventoryGrace: () => mode === 'grace',
    inventoryOpen: () => state.inventoryOpen,
    openInventory() {
      if (!engine.value || switching || mode === 'lock-lost') return
      setMode('inventory')
      beginInventory(engine.value)
    },
    closeInventory() {
      void closeInventory()
    },
  })

  async function closeInventory() {
    if (mode !== 'inventory') return
    setMode('closing')
    engine.value?.release_all_keys()
    const ok = await endInventory(canvas.value)
    if (!modeIs('closing')) return
    if (ok && document.pointerLockElement === canvas.value) {
      state.locked = true
      setMode('grace')
    } else {
      state.locked = false
      setMode('lock-lost')
      engine.value?.set_paused(false)
    }
  }

  watch(
    () => ({ ...settings }),
    () => engine.value && applySettings(engine.value, settings),
  )
  watch([slot, block], () => engine.value && applyBlock(engine.value, block.value))

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
    setMode('idle')
    created.set_paused(BENCH ? false : !input.isLocked())
    applyBlock(created, block.value)
    engine.value = created
    state.ready = true
    pool = BENCH_WORKERS ? startPool(created) : undefined
    loop = new FrameLoop(created, el, {
      fps: (v) => (state.fps = v),
      stats: (v) => (state.stats = v),
      time: (v) => (state.time = v),
      player: (v) => (state.player = v),
      rendered: BENCH ? (now, ms) => benchFrames.push(now, ms) : undefined,
      afterRender: () => pool?.pump(),
    })
    loop.start()
  }

  function startPool(created: Engine) {
    try {
      return new MeshPool(created)
    } catch (e) {
      console.error('mesh workers are unavailable', e)
      return undefined
    }
  }

  function stop() {
    loop?.stop()
    loop = undefined
    pool?.dispose()
    pool = undefined
    setMode('idle')
    if (document.pointerLockElement === canvas.value) document.exitPointerLock()
    engine.value?.free()
    engine.value = undefined
  }

  async function newWorld() {
    if (switching || !engine.value) return
    setSwitching(true)
    try {
      resetHotbar()
      stop()
      await persistence.reset()
      canvasKey.value++
      await nextTick()
      await start(randomSeed())
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
    requestLock: () => requestPointerLock(canvas.value),
    releaseCursor: releasePointerLock,
    newWorld,
  }
}
