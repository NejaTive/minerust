<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import init, { Engine } from '@engine'

const emit = defineEmits<{ fps: [value: number] }>()

const canvas = ref<HTMLCanvasElement>()
// shallowRef: Vue не должен оборачивать wasm-объект в Proxy.
const engine = shallowRef<Engine>()
const error = ref<string>()

let rafId = 0
let resizeObserver: ResizeObserver | undefined

function resize() {
  const el = canvas.value!
  const dpr = window.devicePixelRatio
  el.width = Math.max(1, Math.floor(el.clientWidth * dpr))
  el.height = Math.max(1, Math.floor(el.clientHeight * dpr))
  engine.value?.resize(el.width, el.height)
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
  engine.value?.free()
})
</script>

<template>
  <canvas ref="canvas" class="game" />
  <div v-if="error" class="error">Не удалось запустить движок: {{ error }}</div>
</template>

<style scoped>
.game {
  display: block;
  width: 100vw;
  height: 100vh;
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
