<script setup lang="ts">
import { ref } from 'vue'
import { gameState } from '../engine/state'
import { useEngine } from '../engine/useEngine'

const props = defineProps<{ worldId: string }>()
const emit = defineEmits<{ 'canvas-click': [] }>()

const canvas = ref<HTMLCanvasElement>()

const { requestLock, releaseCursor, save } = useEngine(canvas, props.worldId)

function onCanvasClick() {
  if (gameState.locked || gameState.inventoryOpen || gameState.consoleOpen) return
  emit('canvas-click')
}

defineExpose({ requestLock, releaseCursor, save })
</script>

<template>
  <canvas ref="canvas" class="game" @click="onCanvasClick" @contextmenu.prevent />
  <div v-if="gameState.ready && gameState.locked" class="crosshair" />
  <div v-if="gameState.error" class="error">Не удалось запустить движок: {{ gameState.error }}</div>
</template>

<style scoped>
.game {
  display: block;
  width: 100vw;
  height: 100vh;
}
.crosshair {
  position: fixed;
  top: 50%;
  left: 50%;
  width: calc(15 * var(--gui-u));
  height: calc(15 * var(--gui-u));
  transform: translate(-50%, -50%);
  pointer-events: none;
  mix-blend-mode: difference;
  background: var(--gui-icons) 0 0 / calc(256 * var(--gui-u)) calc(256 * var(--gui-u)) no-repeat;
  image-rendering: pixelated;
}
.error {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: var(--px-fs);
  background: #400;
  padding: 16px;
}
</style>
