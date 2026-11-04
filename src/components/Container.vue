<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  useTemplateRef,
  watch,
} from 'vue'
import type { Engine } from '@engine'
import { paintBlockIconsIn } from '../engine/blockIcon'
import { blockLabel } from '../engine/info'
import { texturePacks } from '../engine/useTexturePack'
import { useSettings } from '../settings/useSettings'
import {
  CHEST_COLUMNS,
  changedSlots,
  clearSlot,
  clickSlot,
  containerLayout,
  onContainerClose,
  returnCursor,
  shiftClick,
  type Area,
  type ContainerState,
} from '../ui/container'

const props = defineProps<{ engine: Engine }>()

const { slots: hotbarSlots, putInSlot } = useSettings()

const state = reactive<ContainerState>({
  chest: props.engine.containerSlots().map((name) => name || null),
  hotbar: [...hotbarSlots],
  cursor: null,
})
const layout = computed(() => containerLayout(state.chest.length))
const root = useTemplateRef<HTMLElement>('root')
const mouse = ref({ x: 0, y: 0 })

let synced = [...state.chest]
let syncedHotbar = [...state.hotbar]

function sync() {
  for (const i of changedSlots(synced, state.chest)) {
    props.engine.setContainerSlot(i, state.chest[i] ?? '')
  }
  synced = [...state.chest]
  for (const i of changedSlots(syncedHotbar, state.hotbar)) putInSlot(i, state.hotbar[i] ?? null)
  syncedHotbar = [...state.hotbar]
}

function click(e: MouseEvent, area: Area, index: number) {
  if (e.shiftKey) shiftClick(state, area, index)
  else clickSlot(state, area, index)
  sync()
}

function clear(e: MouseEvent, area: Area, index: number) {
  e.preventDefault()
  clearSlot(state, area, index)
  sync()
}

onContainerClose(() => {
  returnCursor(state)
  sync()
})
onBeforeUnmount(() => onContainerClose(undefined))

const paint = () => paintBlockIconsIn(root.value)
onMounted(paint)
watch(
  [state, () => texturePacks.state.revision],
  async () => {
    await nextTick()
    paint()
  },
  { deep: true },
)

const position = (index: number, top: number) => ({
  left: `calc(${7 + (index % CHEST_COLUMNS) * 18} * var(--gui-u))`,
  top: `calc(${top + Math.floor(index / CHEST_COLUMNS) * 18} * var(--gui-u))`,
})
</script>

<template>
  <div
    ref="root"
    class="overlay"
    role="dialog"
    aria-modal="true"
    :aria-label="layout.title"
    @mousemove="mouse = { x: $event.clientX, y: $event.clientY }"
  >
    <div class="window" :style="{ height: `calc(${layout.height} * var(--gui-u))` }">
      <div class="top" :style="{ height: `calc(${layout.topHeight} * var(--gui-u))` }" />
      <div class="bottom" :style="{ top: `calc(${layout.topHeight} * var(--gui-u))` }" />
      <span class="label" :style="{ top: `calc(6 * var(--gui-u))` }">{{ layout.title }}</span>
      <span class="label" :style="{ top: `calc(${layout.inventoryLabelY} * var(--gui-u))` }">
        Инвентарь
      </span>
      <div
        v-for="(name, index) in state.chest"
        :key="`c${index}`"
        class="slot"
        data-area="chest"
        :style="position(index, 17)"
        :title="name ? blockLabel(name) : undefined"
        @click="click($event, 'chest', index)"
        @contextmenu="clear($event, 'chest', index)"
      >
        <canvas v-if="name" :data-block="name" />
      </div>
      <div
        v-for="(name, index) in state.hotbar"
        :key="`h${index}`"
        class="slot"
        data-area="hotbar"
        :style="position(index, layout.hotbarY - 1)"
        :title="name ? blockLabel(name) : undefined"
        @click="click($event, 'hotbar', index)"
        @contextmenu="clear($event, 'hotbar', index)"
      >
        <canvas v-if="name" :data-block="name" />
      </div>
    </div>
    <canvas
      v-if="state.cursor"
      :data-block="state.cursor"
      class="cursor"
      :style="{ left: `${mouse.x}px`, top: `${mouse.y}px` }"
    />
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
.window {
  position: relative;
  width: calc(176 * var(--gui-u));
}
.top,
.bottom {
  position: absolute;
  left: 0;
  width: 100%;
  background-image: var(--gui-container);
  background-size: calc(256 * var(--gui-u)) calc(256 * var(--gui-u));
  background-repeat: no-repeat;
  image-rendering: pixelated;
}
.top {
  top: 0;
  background-position: 0 0;
}
.bottom {
  height: calc(96 * var(--gui-u));
  background-position: 0 calc(-126 * var(--gui-u));
}
.label {
  position: absolute;
  left: calc(8 * var(--gui-u));
  font-size: var(--px-fs-s);
  line-height: 1;
  pointer-events: none;
}
.slot {
  position: absolute;
  display: flex;
  align-items: center;
  justify-content: center;
  width: calc(18 * var(--gui-u));
  height: calc(18 * var(--gui-u));
  cursor: pointer;
}
.slot:hover::before {
  position: absolute;
  inset: var(--gui-u);
  content: '';
  background: rgb(255 255 255 / 0.5);
}
canvas {
  width: calc(16 * var(--gui-u));
  height: calc(16 * var(--gui-u));
  image-rendering: pixelated;
  pointer-events: none;
}
.cursor {
  position: fixed;
  z-index: 16;
  transform: translate(-50%, -50%);
}
</style>
