<script setup lang="ts">
import { computed, nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { BLOCK_ICON_SIZE, paintBlockIconsIn } from '../engine/blockIcon'
import { blockLabel, engineInfo } from '../engine/info'
import { useSettings } from '../settings/useSettings'

const { slots, slot, putInSlot, swapSlots, clearSlot } = useSettings()

const root = useTemplateRef<HTMLElement>('root')

const palette = computed(() => engineInfo().blocks.filter((b) => b.placeable))

const paint = () => paintBlockIconsIn(root.value)

onMounted(paint)
watch(
  [palette, slots],
  async () => {
    await nextTick()
    paint()
  },
  { deep: true },
)

function putSelected(name: string) {
  putInSlot(slot.value, name)
}

function selectSlot(index: number) {
  slot.value = index
}

function onPaletteDragStart(e: DragEvent, name: string) {
  e.dataTransfer?.setData('text/plain', `block:${name}`)
  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy'
}

function onHotbarDragStart(e: DragEvent, index: number) {
  e.dataTransfer?.setData('text/plain', `slot:${index}`)
  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
}

function onHotbarDragOver(e: DragEvent) {
  e.preventDefault()
  if (!e.dataTransfer) return
  e.dataTransfer.dropEffect = e.dataTransfer.effectAllowed === 'copy' ? 'copy' : 'move'
}

function onHotbarDrop(e: DragEvent, index: number) {
  e.preventDefault()
  const raw = e.dataTransfer?.getData('text/plain') ?? ''
  const blockMatch = /^block:(.+)$/.exec(raw)
  if (blockMatch) {
    putInSlot(index, blockMatch[1]!)
    return
  }
  const slotMatch = /^slot:(\d+)$/.exec(raw)
  if (slotMatch) {
    swapSlots(Number(slotMatch[1]), index)
  }
}

function onHotbarContextMenu(e: MouseEvent, index: number) {
  e.preventDefault()
  clearSlot(index)
}
</script>

<template>
  <div ref="root" class="overlay" role="dialog" aria-modal="true" aria-label="Инвентарь">
    <div class="panel px-frame">
      <h1>Инвентарь</h1>
      <p class="hint">
        Клик — в выбранный слот · перетащите в слот · ПКМ по слоту — очистить · E / Esc — закрыть
      </p>
      <div class="palette">
        <button
          v-for="block in palette"
          :key="block.name"
          type="button"
          class="cell px-slot"
          :title="block.label"
          draggable="true"
          @click="putSelected(block.name)"
          @dragstart="onPaletteDragStart($event, block.name)"
        >
          <canvas :data-block="block.name" :width="BLOCK_ICON_SIZE" :height="BLOCK_ICON_SIZE" />
        </button>
      </div>
      <div class="hotbar-row">
        <div
          v-for="(name, index) in slots"
          :key="index"
          class="slot px-slot"
          :class="{ selected: index === slot }"
          :title="name ? blockLabel(name) : undefined"
          draggable="true"
          @dragstart="onHotbarDragStart($event, index)"
          @dragover="onHotbarDragOver"
          @drop="onHotbarDrop($event, index)"
          @contextmenu="onHotbarContextMenu($event, index)"
          @click="selectSlot(index)"
        >
          <canvas
            v-if="name"
            :data-block="name"
            :width="BLOCK_ICON_SIZE"
            :height="BLOCK_ICON_SIZE"
          />
          <span class="number">{{ index + 1 }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 15;
  display: grid;
  place-items: center;
  padding: 16px;
  overflow: auto;
  color: var(--px-text);
  text-shadow: 2px 2px 0 var(--px-shadow);
  background: rgb(0 0 0 / 0.45);
}
.panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  box-sizing: border-box;
  width: min(560px, 100%);
  padding: 20px;
}
h1 {
  margin: 0;
  font-size: var(--px-fs);
  font-weight: 700;
  text-align: center;
}
.hint {
  margin: 0;
  font-size: var(--px-fs-s);
  line-height: 1.35;
  text-align: center;
}
.palette {
  display: grid;
  grid-template-columns: repeat(auto-fill, 48px);
  gap: 4px;
  justify-content: center;
}
.cell {
  width: 48px;
  height: 48px;
  margin: 0;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: grab;
}
.cell:hover:not(:disabled) {
  background: rgb(255 255 255 / 0.2);
  border-color: #fff;
}
.hotbar-row {
  display: flex;
  gap: 4px;
  justify-content: center;
  padding: 4px;
  background: rgb(0 0 0 / 0.35);
}
.slot {
  position: relative;
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: grab;
}
canvas {
  width: 32px;
  height: 32px;
  image-rendering: pixelated;
  pointer-events: none;
}
.number {
  position: absolute;
  left: 0;
  top: -2px;
  color: #fff;
  font-size: var(--px-fs-s);
  line-height: 1;
  text-shadow: 2px 2px 0 #000;
  pointer-events: none;
}
</style>
