<script setup lang="ts">
import { computed, nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { paintBlockIconsIn } from '../engine/blockIcon'
import { blockLabel, engineInfo } from '../engine/info'
import { texturePacks } from '../engine/useTexturePack'
import { useSettings } from '../settings/useSettings'

const { slots, slot, putInSlot, swapSlots, clearSlot } = useSettings()

const root = useTemplateRef<HTMLElement>('root')

const palette = computed(() => engineInfo().blocks.filter((b) => b.placeable))

const paint = () => paintBlockIconsIn(root.value)

onMounted(paint)
watch(
  [palette, slots, () => texturePacks.state.revision],
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
    <div class="panel gui-panel">
      <h1>Инвентарь</h1>
      <p class="hint">
        Клик — в выбранный слот · перетащите в слот · ПКМ по слоту — очистить · E / Esc — закрыть
      </p>
      <div class="palette">
        <button
          v-for="block in palette"
          :key="block.name"
          type="button"
          class="cell gui-slot"
          :title="block.label"
          draggable="true"
          @click="putSelected(block.name)"
          @dragstart="onPaletteDragStart($event, block.name)"
        >
          <canvas :data-block="block.name" />
        </button>
      </div>
      <div class="hotbar-row">
        <div
          v-for="(name, index) in slots"
          :key="index"
          class="slot gui-slot"
          :class="{ selected: index === slot }"
          :title="name ? blockLabel(name) : undefined"
          draggable="true"
          @dragstart="onHotbarDragStart($event, index)"
          @dragover="onHotbarDragOver"
          @drop="onHotbarDrop($event, index)"
          @contextmenu="onHotbarContextMenu($event, index)"
          @click="selectSlot(index)"
        >
          <canvas v-if="name" :data-block="name" />
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
  color: #404040;
  background: rgb(0 0 0 / 0.45);
}
.panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  box-sizing: border-box;
  width: calc(182 * var(--gui-u));
  padding: calc(6 * var(--gui-u));
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
  grid-template-columns: repeat(9, calc(18 * var(--gui-u)));
}
.hotbar-row {
  display: flex;
}
.cell,
.slot {
  position: relative;
  width: calc(18 * var(--gui-u));
  height: calc(18 * var(--gui-u));
  margin: 0;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: grab;
}
.cell:hover::before {
  position: absolute;
  inset: var(--gui-u);
  content: '';
  background: rgb(255 255 255 / 0.5);
}
.slot.selected {
  z-index: 1;
}
.slot.selected::after {
  position: absolute;
  top: calc(-3 * var(--gui-u));
  left: calc(-3 * var(--gui-u));
  width: calc(24 * var(--gui-u));
  height: calc(24 * var(--gui-u));
  content: '';
  background: var(--gui-widgets) 0 calc(-22 * var(--gui-u)) / calc(256 * var(--gui-u))
    calc(256 * var(--gui-u)) no-repeat;
  image-rendering: pixelated;
  pointer-events: none;
}
canvas {
  width: calc(16 * var(--gui-u));
  height: calc(16 * var(--gui-u));
  image-rendering: pixelated;
  pointer-events: none;
}
.number {
  position: absolute;
  right: 0;
  bottom: -2px;
  color: #fff;
  font-size: var(--px-fs-s);
  line-height: 1;
  text-shadow: 2px 2px 0 #000;
  pointer-events: none;
}
</style>
