<script setup lang="ts">
import { nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { paintBlockIconsIn } from '../engine/blockIcon'
import { blockLabel } from '../engine/info'
import { texturePacks } from '../engine/useTexturePack'
import { useSettings } from '../settings/useSettings'

defineProps<{ slotIndex: number }>()

const { slots } = useSettings()
const root = useTemplateRef<HTMLElement>('root')

const paint = () => paintBlockIconsIn(root.value)

onMounted(paint)
watch(
  [slots, () => texturePacks.state.revision],
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
      <canvas v-if="name" :data-block="name" />
      <span class="number">{{ index + 1 }}</span>
    </div>
  </div>
</template>

<style scoped>
.hotbar {
  position: fixed;
  left: 50%;
  bottom: 8px;
  transform: translateX(-50%);
  box-sizing: border-box;
  display: flex;
  gap: calc(4 * var(--gui-u));
  width: calc(182 * var(--gui-u));
  height: calc(22 * var(--gui-u));
  padding: calc(3 * var(--gui-u));
  background: var(--gui-widgets) 0 0 / calc(256 * var(--gui-u)) calc(256 * var(--gui-u)) no-repeat;
  image-rendering: pixelated;
  pointer-events: none;
  z-index: 10;
}
.slot {
  position: relative;
  width: calc(16 * var(--gui-u));
  height: calc(16 * var(--gui-u));
}
.slot.selected {
  z-index: 1;
}
.slot.selected::after {
  position: absolute;
  top: calc(-4 * var(--gui-u));
  left: calc(-4 * var(--gui-u));
  width: calc(24 * var(--gui-u));
  height: calc(24 * var(--gui-u));
  content: '';
  background: var(--gui-widgets) 0 calc(-22 * var(--gui-u)) / calc(256 * var(--gui-u))
    calc(256 * var(--gui-u)) no-repeat;
  image-rendering: pixelated;
}
canvas {
  width: 100%;
  height: 100%;
  image-rendering: pixelated;
}
.number {
  position: absolute;
  right: 0;
  bottom: -2px;
  color: #fff;
  font-size: var(--px-fs-s);
  line-height: 1;
  text-shadow: 2px 2px 0 #000;
}
</style>
