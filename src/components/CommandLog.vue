<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { visibleLog } from '../console/log'
import { consoleInput, consoleLog } from '../engine/console'
import { gameState } from '../engine/state'

const now = ref(performance.now())
let timer = 0
onMounted(() => (timer = window.setInterval(() => (now.value = performance.now()), 500)))
onBeforeUnmount(() => clearInterval(timer))

const entries = computed(() => visibleLog(consoleLog.entries, now.value, gameState.consoleOpen))
</script>

<template>
  <div v-if="entries.length" class="log" :class="{ open: gameState.consoleOpen }">
    <div
      v-for="entry in entries"
      :key="entry.id"
      class="line"
      :class="{ error: !entry.ok, link: gameState.consoleOpen && entry.suggest }"
      @click="entry.suggest && gameState.consoleOpen && (consoleInput = entry.suggest)"
    >
      {{ entry.text }}
    </div>
  </div>
</template>

<style scoped>
.log {
  position: fixed;
  left: 8px;
  bottom: 96px;
  max-width: min(720px, calc(100vw - 16px));
  z-index: 20;
  pointer-events: none;
  font-size: var(--px-fs-s);
  line-height: 1.4;
  text-shadow: 2px 2px 0 #000;
}
.log.open {
  bottom: 88px;
  padding: 4px 8px;
  background: rgb(0 0 0 / 50%);
}
.line {
  color: var(--px-text);
  white-space: pre-wrap;
}
.line.link {
  cursor: pointer;
  pointer-events: auto;
  text-decoration: underline;
}
.line.link:hover {
  color: var(--px-text-hover);
}
.line.error {
  color: var(--px-red-light);
}
</style>
