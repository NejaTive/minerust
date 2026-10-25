<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import init, { Engine } from '@engine'

const emit = defineEmits<{ fps: [value: number] }>()

const canvas = ref<HTMLCanvasElement>()
// shallowRef: Vue не должен оборачивать wasm-объект в Proxy.
const engine = shallowRef<Engine>()
const error = ref<string>()
// Реактивное состояние pointer lock: мышь захвачена canvas'ом.
const locked = ref(false)

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

function onKeyDown(e: KeyboardEvent) {
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

onMounted(async () => {
  try {
    await init()
    resize()
    engine.value = await Engine.create(canvas.value!)
  } catch (e) {
    error.value = String(e)
    return
  }

  document.addEventListener('pointerlockchange', onLockChange)
  document.addEventListener('mousemove', onMouseMove)
  window.addEventListener('keydown', onKeyDown)
  window.addEventListener('keyup', onKeyUp)
  window.addEventListener('blur', onBlur)

  resizeObserver = new ResizeObserver(resize)
  resizeObserver.observe(canvas.value!)

  let frames = 0
  let lastFpsTime = performance.now()
  const frame = (now: number) => {
    engine.value!.render(now / 1000)
    frames++
    if (now - lastFpsTime >= 1000) {
      emit('fps', frames)
      frames = 0
      lastFpsTime = now
    }
    rafId = requestAnimationFrame(frame)
  }
  rafId = requestAnimationFrame(frame)
})

onBeforeUnmount(() => {
  cancelAnimationFrame(rafId)
  resizeObserver?.disconnect()
  document.removeEventListener('pointerlockchange', onLockChange)
  document.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('keydown', onKeyDown)
  window.removeEventListener('keyup', onKeyUp)
  window.removeEventListener('blur', onBlur)
  if (document.pointerLockElement === canvas.value) document.exitPointerLock()
  engine.value?.free()
})
</script>

<template>
  <canvas ref="canvas" class="game" @click="requestLock" />
  <!-- Подсказка и прицел только когда движок запущен; клики проходят на canvas. -->
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
