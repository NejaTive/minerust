<script setup lang="ts">
import { API_LABELS } from '../settings'
import type { ApiInfo } from './GameCanvas.vue'

defineProps<{ fps: number; api?: ApiInfo; notice?: string }>()
</script>

<template>
  <div class="hud">
    <div>MineRust · {{ fps }} FPS</div>
    <div v-if="api">
      {{ API_LABELS[api.backend] }}<template v-if="api.fallback"> (WebGPU недоступен)</template>
    </div>
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
}
.notice {
  margin-top: 4px;
  color: #ffd75e;
}
</style>
