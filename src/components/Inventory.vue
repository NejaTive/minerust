<script setup lang="ts">
import { computed, nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { BLOCK_ICON_SIZE, paintBlockIconsIn } from '../engine/blockIcon'
import { blockLabel, engineInfo } from '../engine/info'
import { useSettings } from '../settings/useSettings'

const { slots, slot, putInSlot, swapSlots, clearSlot } = useSettings()

const root = useTemplateRef<HTMLElement>('root')

/** Размещаемые блоки из движка; воздух и прочее неразмещаемое — вне палитры. */
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
  // getData в dragover недоступен; effectAllowed выставили в dragstart.
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
    <div class="panel">
      <h1>Инвентарь</h1>
      <p class="hint">
        Клик — в выбранный слот · перетащите в слот · ПКМ по слоту — очистить · E / Esc — закрыть
      </p>
      <div class="palette">
        <button
          v-for="block in palette"
          :key="block.name"
          type="button"
          class="cell"
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
          class="slot"
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
  color: #f2f4f7;
  font-size: 15px;
  background: rgba(8, 10, 14, 0.45);
}
.panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  box-sizing: border-box;
  width: min(480px, 100%);
  padding: 20px;
  background: #1b1f26;
  border: 1px solid #343b47;
  border-radius: 12px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
}
h1 {
  margin: 0;
  font-size: 22px;
  font-weight: 600;
  text-align: center;
}
.hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.35;
  color: #a8b0bd;
  text-align: center;
}
.palette {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(48px, 1fr));
  gap: 6px;
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
  background: rgb(0 0 0 / 0.35);
  border: 2px solid rgb(255 255 255 / 0.2);
  border-radius: 4px;
  color: inherit;
}
.cell:hover {
  border-color: #fff;
  background: rgb(255 255 255 / 0.12);
}
.hotbar-row {
  display: flex;
  gap: 4px;
  justify-content: center;
  padding: 4px;
  background: rgb(0 0 0 / 0.35);
  border-radius: 4px;
}
.slot {
  position: relative;
  width: 48px;
  height: 48px;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: grab;
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
  image-rendering: pixelated;
  pointer-events: none;
}
.number {
  position: absolute;
  left: 2px;
  top: 0;
  color: #fff;
  font-size: 10px;
  line-height: 1.2;
  text-shadow: 1px 1px 0 #000;
  pointer-events: none;
}
</style>
