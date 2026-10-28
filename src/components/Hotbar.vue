<script setup lang="ts">
import { onMounted, useTemplateRef } from 'vue'
import { block_icon } from '@engine'
import { blockLabel } from '../engine/info'
import { HOTBAR } from '../settings/hotbar'

defineProps<{ slotIndex: number }>()

/** Сторона тайла в пикселях; иконка — тот же тайл, что на боковой грани блока в мире. */
const ICON_SIZE = 16

const root = useTemplateRef<HTMLElement>('root')

// wasm к этому моменту уже инициализирован: хотбар монтируется после старта движка.
// Иконки рисуются по data-block, а не по порядку ref'ов в v-for: Vue его не гарантирует.
onMounted(() => {
  root.value?.querySelectorAll('canvas').forEach((canvas) => {
    const pixels = block_icon(canvas.dataset.block ?? '')
    if (!pixels) return
    canvas
      .getContext('2d')
      ?.putImageData(new ImageData(new Uint8ClampedArray(pixels), ICON_SIZE, ICON_SIZE), 0, 0)
  })
})
</script>

<template>
  <div ref="root" class="hotbar">
    <div
      v-for="(name, index) in HOTBAR"
      :key="index"
      class="slot"
      :class="{ selected: index === slotIndex }"
      :title="name ? blockLabel(name) : undefined"
    >
      <canvas v-if="name" :data-block="name" :width="ICON_SIZE" :height="ICON_SIZE" />
      <span class="number">{{ index + 1 }}</span>
    </div>
  </div>
</template>

<style scoped>
.hotbar {
  position: fixed;
  left: 50%;
  bottom: 16px;
  transform: translateX(-50%);
  display: flex;
  gap: 4px;
  padding: 4px;
  background: rgb(0 0 0 / 0.45);
  border-radius: 4px;
  pointer-events: none;
  /* Под HUD (20) и меню паузы, над canvas. */
  z-index: 10;
}
.slot {
  position: relative;
  width: 48px;
  height: 48px;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgb(0 0 0 / 0.35);
  border: 3px solid rgb(255 255 255 / 0.25);
}
.slot.selected {
  border-color: #fff;
  background: rgb(255 255 255 / 0.2);
}
canvas {
  width: 32px;
  height: 32px;
  /* Без сглаживания: иконка — те же пиксели, что у блока в мире. */
  image-rendering: pixelated;
}
.number {
  position: absolute;
  left: 2px;
  top: 0;
  color: #fff;
  font-size: 10px;
  line-height: 1.2;
  text-shadow: 1px 1px 0 #000;
}
</style>
