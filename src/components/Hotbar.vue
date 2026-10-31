<script setup lang="ts">
import { nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { BLOCK_ICON_SIZE, paintBlockIconsIn } from '../engine/blockIcon'
import { blockLabel } from '../engine/info'
import { useSettings } from '../settings/useSettings'

defineProps<{ slotIndex: number }>()

const { slots } = useSettings()
const root = useTemplateRef<HTMLElement>('root')

const paint = () => paintBlockIconsIn(root.value)

onMounted(paint)
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
      class="slot px-slot"
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
  pointer-events: none;
  z-index: 10;
}
.slot {
  position: relative;
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
}
canvas {
  width: 32px;
  height: 32px;
  image-rendering: pixelated;
}
.number {
  position: absolute;
  left: 0;
  top: -2px;
  color: #fff;
  font-size: var(--px-fs-s);
  line-height: 1;
  text-shadow: 2px 2px 0 #000;
}
</style>
