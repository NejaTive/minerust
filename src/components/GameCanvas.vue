<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import init, { Engine } from '@engine'
import {
  HOTBAR,
  MAX_RENDER_DISTANCE,
  MIN_RENDER_DISTANCE,
  type Settings,
} from '../settings'
import {
  BENCH,
  BENCH_POINTS,
  BENCH_SEED,
  BENCH_TIME,
  FrameRecorder,
  runBench,
} from '../bench'
import { NoWebGpuError, hasWebGpu } from '../gpu'
import { type WorldStats, parseStats } from '../stats'
import { type SavedColumn, type WorldMeta, type WorldStorage, openStorage } from '../storage'

export type { WorldStats }

export interface TimeInfo {
  time: number
  phase: string
}

export interface PlayerInfo {
  mode: 'walking' | 'flying' | 'noclip'
  onGround: boolean
}

const props = defineProps<{ settings: Settings; slot: number }>()

const emit = defineEmits<{
  fps: [value: number]
  stats: [value: WorldStats]
  /** Движок создан и рисует: можно показывать хотбар. */
  ready: []
  /** WebGPU нет: `reason` — текст ошибки движка, если браузер его заявил, но адаптера нет. */
  'no-webgpu': [reason?: string]
  notice: [text: string]
  'render-distance': [columns: number]
  /** Игрок выбрал слот хотбара (клавиша, колесо): v-model:slot. */
  'update:slot': [slot: number]
  player: [info: PlayerInfo]
  /** Время суток 0..1 и фаза (`day`, `dusk`, `night`, `dawn`), раз в секунду. */
  time: [info: TimeInfo]
  /** Курсор захвачен (игра идёт) или отпущен (пауза, меню). */
  lock: [locked: boolean]
  /** Движок грузится или пересоздаётся, мир менять нельзя. */
  busy: [busy: boolean]
  /** Нажата N: подтверждение «Новый мир» показывает меню. */
  'ask-new-world': []
}>()

const canvas = ref<HTMLCanvasElement>()
// Смена ключа пересоздаёт <canvas>: новый мир начинается на чистом контексте, а не на
// переконфигурированной поверхности прежнего движка.
const canvasKey = ref(0)
// shallowRef: Vue не должен оборачивать wasm-объект в Proxy.
const engine = shallowRef<Engine>()
const error = ref<string>()
const locked = ref(false)

const benchFrames = new FrameRecorder()
// Блокирует N, пока движок пересоздаётся; до конца загрузки true.
let switching = true
function setSwitching(value: boolean) {
  switching = value
  emit('busy', value)
}
let disposed = false
let rafId = 0
let resizeObserver: ResizeObserver | undefined

function resize() {
  const el = canvas.value!
  const dpr = window.devicePixelRatio
  el.width = Math.max(1, Math.floor(el.clientWidth * dpr))
  el.height = Math.max(1, Math.floor(el.clientHeight * dpr))
  engine.value?.resize(el.width, el.height)
}

// Клавиши, для которых гасим действие браузера по умолчанию (скролл страницы и т.п.).
const GAME_KEYS = new Set(['Space', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'])

/**
 * Захватывает курсор; вызывать из обработчика действия пользователя (клик по кнопке меню).
 * Возвращает, удалось ли: Chrome около секунды после выхода по Esc отказывает.
 */
async function requestLock(): Promise<boolean> {
  const el = canvas.value
  if (!el) return false
  try {
    // Старые браузеры возвращают undefined, новые — Promise, который может реджектиться.
    await (el.requestPointerLock() as Promise<void> | undefined)
  } catch {
    return false
  }
  return true
}

function releaseCursor() {
  if (document.pointerLockElement) document.exitPointerLock()
}

// Пауза следует за курсором: Esc браузер забирает себе и до страницы не доводит, так что
// единственный надёжный сигнал выхода — это событие.
function onLockChange() {
  locked.value = document.pointerLockElement === canvas.value
  engine.value?.set_paused(!locked.value)
  if (!locked.value) {
    engine.value?.release_all_keys()
    void save()
  }
  emit('lock', locked.value)
}

/** Настройки из меню — в движок. Идемпотентно: вызывается и после каждого `Engine.create`. */
function applySettings(target: Engine) {
  const s = props.settings
  target.set_render_distance(s.renderDistance)
  target.set_ambient_occlusion(s.ambientOcclusion)
  target.set_mouse_sensitivity(s.mouseSensitivity)
  target.set_fov(s.fov)
  // Замеры кадра нужны только отладочному HUD и режиму замера.
  target.set_profiling(s.showDebug)
}

watch(
  () => ({ ...props.settings }),
  () => {
    if (engine.value) applySettings(engine.value)
  },
)

function onMouseMove(e: MouseEvent) {
  if (locked.value) engine.value?.mouse_move(e.movementX, e.movementY)
}

// Только при уже захваченном курсоре: клик, который захватывает курсор, блок не ломает.
function onMouseDown(e: MouseEvent) {
  if (locked.value) engine.value?.mouse_button(e.button, true)
}

// Отпускание передаём всегда: курсор могли отпустить посреди удержания.
function onMouseUp(e: MouseEvent) {
  engine.value?.mouse_button(e.button, false)
}

/** Выбранный слот — в движок. Слот живёт в App, а не в движке: движок пересоздаётся (новый мир). */
function applySlot(target: Engine) {
  // Пустая строка — пустая рука: ПКМ ничего не ставит.
  target.select_block(HOTBAR[props.slot] ?? '')
}

watch(
  () => props.slot,
  () => {
    if (engine.value) applySlot(engine.value)
  },
)

/** Порог прокрутки, пикселей, на один слот. Колесо мыши шлёт за щелчок 100–120, тачпад — десятки
 * мелких событий за жест: накопление не даёт одному жесту пролистать хотбар. */
const WHEEL_STEP = 100
const WHEEL_LINE_PX = 40
const WHEEL_PAGE_PX = 800
let wheelAccum = 0

function onWheel(e: WheelEvent) {
  if (!locked.value) return
  // Колесо при захваченном курсоре не должно масштабировать или листать страницу.
  e.preventDefault()
  const unit = e.deltaMode === 1 ? WHEEL_LINE_PX : e.deltaMode === 2 ? WHEEL_PAGE_PX : 1
  const delta = e.deltaY * unit
  // Смена направления сбрасывает остаток, иначе первый шаг назад съедался бы им.
  if (Math.sign(delta) !== Math.sign(wheelAccum)) wheelAccum = 0
  wheelAccum += delta
  if (Math.abs(wheelAccum) < WHEEL_STEP) return
  // Вниз — следующий слот, по кругу; за одно событие — не больше одного слота.
  const step = Math.sign(wheelAccum)
  wheelAccum = 0
  emit('update:slot', (props.slot + step + HOTBAR.length) % HOTBAR.length)
}

/** Сид мира, пока нет сохранения: первый запуск даёт привычный рельеф. */
const DEFAULT_SEED = 1337
const AUTOSAVE_MS = 5000

// `null` — IndexedDB недоступен, игра идёт без сохранений.
let storage: WorldStorage | null = null
let formatVersion = 0
// Последние байты каждой изменённой колонки текущего мира (и прочитанных из базы).
const worldColumns = new Map<string, SavedColumn>()
// Колонки, снятые с движка, но ещё не записанные. При ошибке записи остаются здесь
// и уходят со следующим сохранением: `drain_dirty` второй раз их не отдаст.
const unsaved = new Map<string, SavedColumn>()
// Последнее начатое сохранение; IndexedDB выполняет их по очереди, так что его
// завершение означает, что записаны и все предыдущие.
let saving = Promise.resolve()
let saveFailed = false
let autosaveId = 0

function columnKey(column: SavedColumn): string {
  return `${column.x},${column.z}`
}

/** Снимает с движка несохранённые колонки в `worldColumns` и `unsaved`. */
function collectColumns(current: Engine) {
  const keys = current.drain_dirty()
  for (let i = 0; i < keys.length; i += 2) {
    const x = keys[i]
    const z = keys[i + 1]
    const bytes = current.export_column(x, z)
    if (!bytes) continue
    const column = { x, z, bytes }
    worldColumns.set(columnKey(column), column)
    unsaved.set(columnKey(column), column)
  }
}

/**
 * Сохраняет несохранённые колонки и игрока (позицию — каждый раз, даже без правок).
 * Состояние снимается синхронно, запись идёт в фоне; ошибки не пробрасываются.
 */
function save(): Promise<void> {
  const current = engine.value
  const store = storage
  if (!current || !store) return saving
  collectColumns(current)
  const meta: WorldMeta = {
    version: formatVersion,
    seed: current.seed(),
    player: Array.from(current.camera_state()),
    time: current.time_of_day(),
  }
  const columns = [...unsaved.values()]
  saving = store.save(meta, columns).then(
    () => {
      saveFailed = false
      // Сравнение по ссылке: пока шла запись, колонку могли снять с движка заново.
      for (const column of columns) {
        if (unsaved.get(columnKey(column)) === column) unsaved.delete(columnKey(column))
      }
    },
    (e: unknown) => {
      console.warn('world save failed', e)
      if (!saveFailed) emit('notice', 'Не удалось сохранить мир')
      saveFailed = true
    },
  )
  return saving
}

function onVisibilityChange() {
  if (document.visibilityState === 'hidden') void save()
}

// Уход со страницы: запись «как получится», ждать её браузер не станет.
function onPageHide() {
  void save()
}

function isCompatible(meta: WorldMeta): boolean {
  return (
    meta.version === formatVersion &&
    Number.isInteger(meta.seed) &&
    Array.isArray(meta.player) &&
    meta.player.length === 6 &&
    meta.player.every(Number.isFinite)
  )
}

function randomSeed(): number {
  return crypto.getRandomValues(new Int32Array(1))[0]
}

/**
 * Очищает базу. Если не вышло, сохранения выключаются: иначе старые колонки
 * подмешались бы к новому миру при следующей загрузке.
 */
async function clearStorage() {
  try {
    await storage?.clear()
  } catch (e) {
    console.warn('world clear failed', e)
    storage = null
  }
}

interface WorldStart {
  seed: number
  player?: number[]
  time?: number
}

/** Открывает хранилище и читает сохранённый мир в `worldColumns`. */
async function loadWorld(): Promise<WorldStart> {
  storage = await openStorage()
  if (!storage) {
    emit('notice', 'Сохранения недоступны — мир не сохранится')
    return { seed: DEFAULT_SEED }
  }
  let saved
  try {
    saved = await storage.load()
  } catch (e) {
    // Не прочитали — не пишем поверх: в базе мог остаться чужой мир.
    console.warn('world load failed', e)
    storage = null
    emit('notice', 'Сохранение не читается — мир не сохранится')
    return { seed: DEFAULT_SEED }
  }
  if (!saved) return { seed: DEFAULT_SEED }
  if (!isCompatible(saved.meta)) {
    await clearStorage()
    emit('notice', 'Сохранение несовместимо — начат новый мир')
    return { seed: randomSeed() }
  }
  for (const column of saved.columns) worldColumns.set(columnKey(column), column)
  return saved.meta
}

/** Правки из `worldColumns` — в только что созданный движок, до первого кадра. */
function importColumns(created: Engine) {
  let broken = 0
  for (const [key, column] of worldColumns) {
    try {
      if (!Number.isInteger(column.x) || !Number.isInteger(column.z)) throw new Error('bad key')
      if (!(column.bytes instanceof Uint8Array)) throw new Error('bad bytes')
      created.import_column(column.x, column.z, column.bytes)
    } catch (e) {
      console.warn(`saved column ${key} skipped`, e)
      worldColumns.delete(key)
      broken++
    }
  }
  if (broken > 0) emit('notice', `Повреждённые колонки пропущены: ${broken}`)
}

const HOTKEYS = new Set(['KeyN', 'BracketLeft', 'BracketRight'])

/** Хоткеи интерфейса: работают без pointer lock и в движок не попадают. */
function onHotkey(e: KeyboardEvent): boolean {
  if (!HOTKEYS.has(e.code)) return false
  // Cmd+H / Ctrl+G и т.п. — системные сочетания, их не перехватываем.
  if (e.ctrlKey || e.metaKey || e.altKey) return false
  // Дальность при удержании листается автоповтором; N — по одному нажатию.
  if (e.code === 'BracketLeft') changeRenderDistance(-1)
  else if (e.code === 'BracketRight') changeRenderDistance(1)
  else if (!e.repeat && !switching) emit('ask-new-world')
  return true
}

function columnsWord(n: number): string {
  const mod10 = n % 10
  const mod100 = n % 100
  if (mod10 === 1 && mod100 !== 11) return 'колонка'
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return 'колонки'
  return 'колонок'
}

function changeRenderDistance(delta: number) {
  const columns = Math.min(
    MAX_RENDER_DISTANCE,
    Math.max(MIN_RENDER_DISTANCE, props.settings.renderDistance + delta),
  )
  if (columns === props.settings.renderDistance) return
  // Значение лежит в настройках у App; в движок оно попадёт через watch.
  emit('render-distance', columns)
  emit('notice', `Дальность: ${columns} ${columnsWord(columns)}`)
}

function onKeyDown(e: KeyboardEvent) {
  if (onHotkey(e)) return
  if (!locked.value) return
  if (GAME_KEYS.has(e.code)) e.preventDefault()
  if (e.repeat) return
  // Полёт сквозь блоки — отладочный: только при включённой отладке в настройках.
  if (e.code === 'KeyV' && props.settings.showDebug && !e.ctrlKey && !e.metaKey && !e.altKey) {
    engine.value?.toggle_noclip()
    return
  }
  // Перемотка времени суток (×60) — отладочная, пока T зажата.
  if (e.code === 'KeyT' && props.settings.showDebug && !e.ctrlKey && !e.metaKey && !e.altKey) {
    engine.value?.set_time_fast(true)
    return
  }
  // Цифры выбирают слот и в движок как клавиши не попадают.
  const digit = /^Digit([1-9])$/.exec(e.code)
  if (digit) {
    if (!e.ctrlKey && !e.metaKey && !e.altKey) emit('update:slot', Number(digit[1]) - 1)
    return
  }
  engine.value?.set_key(e.code, true)
}

function onKeyUp(e: KeyboardEvent) {
  if (e.code === 'KeyT') engine.value?.set_time_fast(false)
  engine.value?.set_key(e.code, false)
}

function onBlur() {
  engine.value?.set_time_fast(false)
  engine.value?.release_all_keys()
}

function startLoop() {
  let frames = 0
  let lastFpsTime = performance.now()
  let player: PlayerInfo | undefined
  const frame = (now: number) => {
    const current = engine.value!
    const renderStart = BENCH ? performance.now() : 0
    current.render(now / 1000)
    if (BENCH) benchFrames.push(now, performance.now() - renderStart)
    frames++
    // Читаем каждый кадр, чтобы HUD не отставал, но эмитим только изменения.
    const mode = current.player_mode() as PlayerInfo['mode']
    const onGround = current.on_ground()
    if (player?.mode !== mode || player.onGround !== onGround) {
      player = { mode, onGround }
      emit('player', player)
    }
    if (now - lastFpsTime >= 1000) {
      emit('fps', frames)
      emit('stats', parseStats(engine.value!.stats()))
      emit('time', { time: engine.value!.time_of_day(), phase: engine.value!.day_phase() })
      frames = 0
      lastFpsTime = now
    }
    rafId = requestAnimationFrame(frame)
  }
  rafId = requestAnimationFrame(frame)
}

/** `player` — `camera_state()`; без него игрок на спауне. Правки берутся из `worldColumns`. */
async function start(seed: number, player?: ArrayLike<number>, time?: number) {
  resize()
  const camera = player ? Float32Array.from(player) : undefined
  const created = await Engine.create(canvas.value!, camera, seed).catch((e: unknown) => {
    // Движок сообщает об отсутствии адаптера или устройства текстом с префиксом «WebGPU».
    const text = e instanceof Error ? e.message : String(e)
    throw text.startsWith('WebGPU ') ? new NoWebGpuError(text) : e
  })
  if (disposed) {
    created.free()
    return
  }
  importColumns(created)
  if (BENCH) {
    created.set_time_of_day(BENCH_TIME)
    created.set_time_frozen(true)
    window.__benchEngine = created
  } else if (time !== undefined && Number.isFinite(time)) {
    created.set_time_of_day(time)
  }
  applySettings(created)
  // Меню открыто (курсор не захвачен) — новый движок тоже начинает с паузы.
  created.set_paused(BENCH ? false : !locked.value)
  applySlot(created)
  engine.value = created
  emit('ready')

  resizeObserver = new ResizeObserver(resize)
  resizeObserver.observe(canvas.value!)
  startLoop()
}

/** Освобождает движок и всё, что привязано к текущему canvas. */
function stop() {
  cancelAnimationFrame(rafId)
  resizeObserver?.disconnect()
  resizeObserver = undefined
  if (document.pointerLockElement === canvas.value) document.exitPointerLock()
  engine.value?.free()
  engine.value = undefined
}

/** Подтверждение спрашивает меню; здесь мир заменяется сразу. */
async function newWorld() {
  const current = engine.value
  if (switching || !current) return
  setSwitching(true)
  try {
    // Движок останавливаем до очистки, чтобы автосохранение не записало старый мир
    // поверх пустой базы; начатая запись должна закончиться до `clear`.
    stop()
    await saving
    worldColumns.clear()
    unsaved.clear()
    await clearStorage()
    canvasKey.value++
    await nextTick()
    await start(randomSeed())
    // Сразу, а не через AUTOSAVE_MS: иначе перезагрузка вернула бы мир с сидом по умолчанию.
    void save()
    emit('notice', 'Новый мир')
  } catch (e) {
    error.value = String(e)
  } finally {
    setSwitching(false)
  }
}

onMounted(async () => {
  document.addEventListener('pointerlockchange', onLockChange)
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mousedown', onMouseDown)
  document.addEventListener('mouseup', onMouseUp)
  document.addEventListener('wheel', onWheel, { passive: false })
  window.addEventListener('keydown', onKeyDown)
  window.addEventListener('keyup', onKeyUp)
  window.addEventListener('blur', onBlur)
  document.addEventListener('visibilitychange', onVisibilityChange)
  window.addEventListener('pagehide', onPageHide)

  if (!hasWebGpu()) {
    // wasm не грузим: без WebGPU движок всё равно не стартует.
    emit('no-webgpu')
    setSwitching(false)
    return
  }
  try {
    await init()
    formatVersion = Engine.format_version()
    if (BENCH) {
      // Мир в памяти: без хранилища `save` ничего не делает, автосохранения нет.
      await start(BENCH_SEED, BENCH_POINTS[0].camera)
      void runBench(() => engine.value!, benchFrames, canvas.value!)
    } else {
      const world = await loadWorld()
      await start(world.seed, world.player, world.time)
      autosaveId = window.setInterval(save, AUTOSAVE_MS)
    }
  } catch (e) {
    if (e instanceof NoWebGpuError) emit('no-webgpu', e.message)
    else error.value = String(e)
  } finally {
    setSwitching(false)
  }
})

onBeforeUnmount(() => {
  disposed = true
  clearInterval(autosaveId)
  void save()
  stop()
  document.removeEventListener('pointerlockchange', onLockChange)
  document.removeEventListener('mousemove', onMouseMove)
  document.removeEventListener('mousedown', onMouseDown)
  document.removeEventListener('mouseup', onMouseUp)
  document.removeEventListener('wheel', onWheel)
  window.removeEventListener('keydown', onKeyDown)
  window.removeEventListener('keyup', onKeyUp)
  window.removeEventListener('blur', onBlur)
  document.removeEventListener('visibilitychange', onVisibilityChange)
  window.removeEventListener('pagehide', onPageHide)
})

defineExpose({ requestLock, releaseCursor, newWorld })
</script>

<template>
  <canvas
    :key="canvasKey"
    ref="canvas"
    class="game"
    @contextmenu.prevent
  />
  <div v-if="engine && locked" class="crosshair" />
  <div v-if="error" class="error">Не удалось запустить движок: {{ error }}</div>
</template>

<style scoped>
.game {
  display: block;
  width: 100vw;
  height: 100vh;
}
.crosshair {
  position: fixed;
  top: 50%;
  left: 50%;
  width: 20px;
  height: 20px;
  transform: translate(-50%, -50%);
  pointer-events: none;
  mix-blend-mode: difference;
  background:
    linear-gradient(#fff, #fff) center / 2px 100% no-repeat,
    linear-gradient(#fff, #fff) center / 100% 2px no-repeat;
}
.error {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  color: #fff;
  background: #400;
  padding: 16px;
}
</style>
