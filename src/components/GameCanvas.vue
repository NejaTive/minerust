<script setup lang="ts">
import { ref } from 'vue'
import { gameState } from '../engine/state'
import { useEngine } from '../engine/useEngine'

const props = defineProps<{ worldId: string }>()
const emit = defineEmits<{ 'canvas-click': [] }>()

const canvas = ref<HTMLCanvasElement>()

const { requestLock, releaseCursor, save } = useEngine(canvas, props.worldId)

function onCanvasClick() {
  if (gameState.locked || gameState.inventoryOpen) return
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
  width: 20px;
  height: 20px;
  transform: translate(-50%, -50%);
  pointer-events: none;
  mix-blend-mode: difference;
  background:
    linear-gradient(#fff, #fff) center / 2px 100% no-repeat,
    linear-gradient(#fff, #fff) center / 100% 2px no-repeat;
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
