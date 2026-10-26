<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import init, { Engine } from '@engine'
import {
  type GraphicsApi,
  loadGraphicsApi,
  loadRenderDistance,
  saveGraphicsApi,
  saveRenderDistance,
} from '../settings'

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

let preferred = loadGraphicsApi()
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

const HOTKEYS = new Set(['KeyG', 'KeyH', 'BracketLeft', 'BracketRight'])

/** Хоткеи интерфейса: работают без pointer lock и в движок не попадают. */
function onHotkey(e: KeyboardEvent): boolean {
  if (!HOTKEYS.has(e.code)) return false
  // Cmd+H / Ctrl+G и т.п. — системные сочетания, их не перехватываем.
  if (e.ctrlKey || e.metaKey || e.altKey) return false
  // Дальность при удержании листается автоповтором; G и H — по одному нажатию.
  if (e.code === 'BracketLeft') changeRenderDistance(-1)
  else if (e.code === 'BracketRight') changeRenderDistance(1)
  else if (!e.repeat) {
    if (e.code === 'KeyG') void switchApi()
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
  const frame = (now: number) => {
    engine.value!.render(now / 1000)
    frames++
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

async function start(camera?: Float32Array) {
  resize()
  const created = await Engine.create(canvas.value!, preferred, camera)
  if (disposed) {
    created.free()
    return
  }
  created.set_render_distance(loadRenderDistance())
  emit('render-distance', created.render_distance())
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
    stop()
    preferred = target
    saveGraphicsApi(target)
    canvasKey.value++
    await nextTick()
    await start(camera)
  } catch (e) {
    error.value = String(e)
  } finally {
    switching = false
  }
}

onMounted(async () => {
  document.addEventListener('pointerlockchange', onLockChange)
  document.addEventListener('mousemove', onMouseMove)
  window.addEventListener('keydown', onKeyDown)
  window.addEventListener('keyup', onKeyUp)
  window.addEventListener('blur', onBlur)

  switching = true
  try {
    await init()
    await start()
  } catch (e) {
    error.value = String(e)
  } finally {
    switching = false
  }
})

onBeforeUnmount(() => {
  disposed = true
  stop()
  document.removeEventListener('pointerlockchange', onLockChange)
  document.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('keydown', onKeyDown)
  window.removeEventListener('keyup', onKeyUp)
  window.removeEventListener('blur', onBlur)
})
</script>

<template>
  <canvas :key="canvasKey" ref="canvas" class="game" @click="requestLock" />
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
