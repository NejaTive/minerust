<script setup lang="ts">
import { ref } from 'vue'
import { gameState } from '../engine/state'
import { useEngine } from '../engine/useEngine'

const emit = defineEmits<{
  /** Нажата N: подтверждение «Новый мир» показывает меню. */
  'ask-new-world': []
  /** Клик по канвасу при свободном курсоре (после lockLost). */
  'canvas-click': []
}>()

const canvas = ref<HTMLCanvasElement>()
// Смена ключа пересоздаёт <canvas>, см. `newWorld`.
const canvasKey = ref(0)

const { requestLock, releaseCursor, newWorld } = useEngine(canvas, canvasKey, () =>
  emit('ask-new-world'),
)

function onCanvasClick() {
  // При захваченном курсоре клики идут в мир; иначе — попытка продолжить после lockLost.
  if (gameState.locked || gameState.inventoryOpen) return
  emit('canvas-click')
}

defineExpose({ requestLock, releaseCursor, newWorld })
</script>

<template>
  <canvas :key="canvasKey" ref="canvas" class="game" @click="onCanvasClick" @contextmenu.prevent />
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
  background: #400;
  padding: 16px;
}
</style>
