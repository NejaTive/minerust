<script setup lang="ts">
import { API_LABELS, BLOCK_LABELS, type BlockName } from '../settings'
import type { ApiInfo, PlayerInfo, WorldStats } from './GameCanvas.vue'

defineProps<{
  fps: number
  api?: ApiInfo
  stats?: WorldStats
  /** Блок в выбранном слоте; `null` — слот пуст. */
  block: BlockName | null
  player?: PlayerInfo
  notice?: string
  showDebug: boolean
}>()

const ms = (us: number) => (us / 1000).toFixed(2)
</script>

<template>
  <div class="hud">
    <div>MineRust · {{ fps }} FPS</div>
    <div v-if="showDebug && api">
      {{ API_LABELS[api.backend] }}<template v-if="api.fallback"> (WebGPU недоступен)</template>
    </div>
    <div v-if="showDebug && stats">
      Колонки {{ stats.columns }} · с мешем {{ stats.meshedColumns }} · в очереди
      {{ stats.queued }}
    </div>
    <div v-if="showDebug && stats">
      Секции: с мешем {{ stats.meshedSections }} · нарисовано {{ stats.drawnSections }}
    </div>
    <div v-if="showDebug && stats">
      Колонка: генерация {{ ms(stats.generateUs) }} мс · меш {{ ms(stats.meshUs) }} мс
    </div>
    <div v-if="player">
      <template v-if="player.mode === 'flying'">Режим: полёт</template>
      <template v-else-if="player.mode === 'noclip'">Режим: сквозь блоки</template>
      <template v-else>Режим: ходьба<template v-if="player.onGround"> · на земле</template></template>
    </div>
    <div>Блок: {{ block ? BLOCK_LABELS[block] : 'пусто' }}</div>
    <div v-if="notice" class="notice">{{ notice }}</div>
  </div>
</template>

<style scoped>
.hud {
  position: fixed;
  top: 8px;
  left: 8px;
  color: #fff;
  font-size: 14px;
  line-height: 1.4;
  text-shadow: 1px 1px 0 #000;
  pointer-events: none;
  /* Выше затемнения меню: подсказки видны и на паузе. */
  z-index: 20;
}
.notice {
  margin-top: 4px;
  color: #ffd75e;
}
</style>
