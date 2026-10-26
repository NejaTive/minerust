<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import init, { Engine } from '@engine'
import {
  type BlockName,
  type GraphicsApi,
  loadGraphicsApi,
  loadRenderDistance,
  saveGraphicsApi,
  saveRenderDistance,
} from '../settings'
import { type SavedColumn, type WorldMeta, type WorldStorage, openStorage } from '../storage'

/** Порядок полей — как в `Engine.stats()`; время — в микросекундах. */
export interface WorldStats {
  columns: number
  meshedColumns: number
  meshedSections: number
  queued: number
  generateUs: number
  meshUs: number
  /** Секций, прошедших отсечение по пирамиде видимости, в последнем кадре. */
  drawnSections: number
}

export interface PlayerInfo {
  mode: 'walking' | 'flying'
  onGround: boolean
}

export interface ApiInfo {
  backend: GraphicsApi
  /** Просили WebGPU, но браузер его не дал, и движок работает на WebGL2. */
  fallback: boolean
}

const emit = defineEmits<{
  fps: [value: number]
  stats: [value: WorldStats]
  api: [info: ApiInfo]
  notice: [text: string]
  'render-distance': [columns: number]
  block: [name: BlockName]
  player: [info: PlayerInfo]
  'toggle-controls': []
}>()

const canvas = ref<HTMLCanvasElement>()
// Смена ключа пересоздаёт <canvas>: тип контекста (webgpu/webgl2) фиксируется первым
// getContext, и старый canvas другой API уже не отдаст.
const canvasKey = ref(0)
// shallowRef: Vue не должен оборачивать wasm-объект в Proxy.
const engine = shallowRef<Engine>()
const error = ref<string>()
const locked = ref(false)
// Выбор блока живёт здесь, а не в движке: движок пересоздаётся при смене API.
const selectedBlock = ref<BlockName>('grass')

let preferred = loadGraphicsApi()
// Блокирует G и N, пока движок пересоздаётся.
let switching = false
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

function requestLock() {
  // В новых браузерах requestPointerLock возвращает Promise, который может реджектиться.
  const result = canvas.value?.requestPointerLock() as Promise<void> | undefined
  result?.catch(() => {})
}

function onLockChange() {
  locked.value = document.pointerLockElement === canvas.value
  if (!locked.value) engine.value?.release_all_keys()
}

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

const BLOCK_KEYS: Record<string, BlockName> = {
  Digit1: 'grass',
  Digit2: 'dirt',
  Digit3: 'stone',
}

function selectBlock(name: BlockName) {
  selectedBlock.value = name
  engine.value?.select_block(name)
  emit('block', name)
}

/** Сид мира, пока нет сохранения: первый запуск даёт привычный рельеф. */
const DEFAULT_SEED = 1337
const AUTOSAVE_MS = 5000

// `null` — IndexedDB недоступен, игра идёт без сохранений.
let storage: WorldStorage | null = null
let formatVersion = 0
// Последние байты каждой изменённой колонки текущего мира. Из этой копии, а не из базы,
// новый движок получает правки при смене API: так G не теряет их и без IndexedDB.
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

const HOTKEYS = new Set(['KeyG', 'KeyH', 'KeyN', 'BracketLeft', 'BracketRight'])

/** Хоткеи интерфейса: работают без pointer lock и в движок не попадают. */
function onHotkey(e: KeyboardEvent): boolean {
  if (!HOTKEYS.has(e.code)) return false
  // Cmd+H / Ctrl+G и т.п. — системные сочетания, их не перехватываем.
  if (e.ctrlKey || e.metaKey || e.altKey) return false
  // Дальность при удержании листается автоповтором; G, H и N — по одному нажатию.
  if (e.code === 'BracketLeft') changeRenderDistance(-1)
  else if (e.code === 'BracketRight') changeRenderDistance(1)
  else if (!e.repeat) {
    if (e.code === 'KeyG') void switchApi()
    else if (e.code === 'KeyN') void newWorld()
    else emit('toggle-controls')
  }
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
  const current = engine.value
  if (!current) return
  // Движок сам зажимает значение в допустимые пределы.
  current.set_render_distance(current.render_distance() + delta)
  const columns = current.render_distance()
  saveRenderDistance(columns)
  emit('render-distance', columns)
  emit('notice', `Дальность: ${columns} ${columnsWord(columns)}`)
}

function onKeyDown(e: KeyboardEvent) {
  if (onHotkey(e)) return
  if (!locked.value) return
  if (GAME_KEYS.has(e.code)) e.preventDefault()
  if (e.repeat) return
  // Цифры выбирают блок и в движок как клавиши полёта не попадают.
  const block = BLOCK_KEYS[e.code]
  if (block) {
    if (!e.ctrlKey && !e.metaKey && !e.altKey) selectBlock(block)
    return
  }
  engine.value?.set_key(e.code, true)
}

function onKeyUp(e: KeyboardEvent) {
  engine.value?.set_key(e.code, false)
}

function onBlur() {
  engine.value?.release_all_keys()
}

function startLoop() {
  let frames = 0
  let lastFpsTime = performance.now()
  let player: PlayerInfo | undefined
  const frame = (now: number) => {
    const current = engine.value!
    current.render(now / 1000)
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
      const [columns, meshedColumns, meshedSections, queued, generateUs, meshUs, drawnSections] =
        engine.value!.stats()
      emit('stats', {
        columns,
        meshedColumns,
        meshedSections,
        queued,
        generateUs,
        meshUs,
        drawnSections,
      })
      frames = 0
      lastFpsTime = now
    }
    rafId = requestAnimationFrame(frame)
  }
  rafId = requestAnimationFrame(frame)
}

/** `player` — `camera_state()`; без него игрок на спауне. Правки берутся из `worldColumns`. */
async function start(seed: number, player?: ArrayLike<number>) {
  resize()
  const camera = player ? Float32Array.from(player) : undefined
  const created = await Engine.create(canvas.value!, preferred, camera, seed)
  if (disposed) {
    created.free()
    return
  }
  importColumns(created)
  created.set_render_distance(loadRenderDistance())
  emit('render-distance', created.render_distance())
  created.select_block(selectedBlock.value)
  emit('block', selectedBlock.value)
  engine.value = created
  const backend = created.backend() as GraphicsApi
  emit('api', { backend, fallback: preferred === 'webgpu' && backend === 'webgl2' })

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

async function switchApi() {
  const current = engine.value
  if (switching || !current) return
  const target: GraphicsApi = current.backend() === 'webgpu' ? 'webgl2' : 'webgpu'
  switching = true
  try {
    // Проверяем заранее, чтобы не пересоздавать canvas ради того же WebGL2.
    if (target === 'webgpu' && !(await Engine.webgpu_supported())) {
      emit('notice', 'WebGPU недоступен в этом браузере — остаётся WebGL2')
      return
    }
    const camera = current.camera_state()
    const seed = current.seed()
    // Кроме записи в базу, `save` снимает правки в `worldColumns`: оттуда их получит новый движок.
    await save()
    stop()
    preferred = target
    saveGraphicsApi(target)
    canvasKey.value++
    await nextTick()
    await start(seed, camera)
  } catch (e) {
    error.value = String(e)
  } finally {
    switching = false
  }
}

async function newWorld() {
  const current = engine.value
  if (switching || !current) return
  // Пока открыт confirm, keyup до страницы не дойдёт: клавиши залипли бы.
  current.release_all_keys()
  if (!confirm('Начать новый мир? Текущий будет удалён.')) return
  switching = true
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
    switching = false
  }
}

onMounted(async () => {
  document.addEventListener('pointerlockchange', onLockChange)
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mousedown', onMouseDown)
  document.addEventListener('mouseup', onMouseUp)
  window.addEventListener('keydown', onKeyDown)
  window.addEventListener('keyup', onKeyUp)
  window.addEventListener('blur', onBlur)
  document.addEventListener('visibilitychange', onVisibilityChange)
  window.addEventListener('pagehide', onPageHide)

  switching = true
  try {
    await init()
    formatVersion = Engine.format_version()
    const world = await loadWorld()
    await start(world.seed, world.player)
    autosaveId = window.setInterval(save, AUTOSAVE_MS)
  } catch (e) {
    error.value = String(e)
  } finally {
    switching = false
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
  window.removeEventListener('keydown', onKeyDown)
  window.removeEventListener('keyup', onKeyUp)
  window.removeEventListener('blur', onBlur)
  document.removeEventListener('visibilitychange', onVisibilityChange)
  window.removeEventListener('pagehide', onPageHide)
})
</script>

<template>
  <canvas
    :key="canvasKey"
    ref="canvas"
    class="game"
    @click="requestLock"
    @contextmenu.prevent
  />
  <div v-if="engine && !locked" class="hint">Кликни, чтобы играть</div>
  <div v-if="engine && locked" class="crosshair" />
  <div v-if="error" class="error">Не удалось запустить движок: {{ error }}</div>
</template>

<style scoped>
.game {
  display: block;
  width: 100vw;
  height: 100vh;
}
.hint {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 24px;
  text-shadow: 1px 1px 0 #000;
  background: rgba(0, 0, 0, 0.45);
  pointer-events: none;
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
