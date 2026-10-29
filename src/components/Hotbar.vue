<script setup lang="ts">
import { nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { BLOCK_ICON_SIZE, paintBlockIconsIn } from '../engine/blockIcon'
import { blockLabel } from '../engine/info'
import { useSettings } from '../settings/useSettings'

defineProps<{ slotIndex: number }>()

const { slots } = useSettings()
const root = useTemplateRef<HTMLElement>('root')

const paint = () => paintBlockIconsIn(root.value)

// wasm к этому моменту уже инициализирован: хотбар монтируется после старта движка.
onMounted(paint)
// Смена содержимого слота (инвентарь) — перерисовать иконки после обновления DOM.
watch(
  slots,
  async () => {
    await nextTick()
    paint()
  },
  { deep: true },
)
</script>

<template>
  <div ref="root" class="hotbar">
    <div
      v-for="(name, index) in slots"
      :key="index"
      class="slot"
      :class="{ selected: index === slotIndex }"
      :title="name ? blockLabel(name) : undefined"
    >
      <canvas v-if="name" :data-block="name" :width="BLOCK_ICON_SIZE" :height="BLOCK_ICON_SIZE" />
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
