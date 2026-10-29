<script setup lang="ts">
import { ref, useTemplateRef, watch } from 'vue'
import GameCanvas from './components/GameCanvas.vue'
import Hotbar from './components/Hotbar.vue'
import Hud from './components/Hud.vue'
import Inventory from './components/Inventory.vue'
import PauseMenu from './components/PauseMenu.vue'
import type { MenuScreen } from './components/menu/screens'
import WebGpuRequired from './components/WebGpuRequired.vue'
import { BENCH } from './bench/config'
import { useGameState } from './engine/state'
import { useSettings } from './settings/useSettings'

const state = useGameState()
const { settings, limits, slot, block, update } = useSettings()

const game = useTemplateRef('game')
const started = ref(false)
const lockFailed = ref(false)
const screen = ref<MenuScreen>('main')

watch(
  () => state.locked,
  (locked) => {
    if (!locked) return
    started.value = true
    lockFailed.value = false
    screen.value = 'main'
  },
)

async function play() {
  lockFailed.value = !(await game.value?.requestLock())
}

async function resumeFromLockLost() {
  if (!state.lockLost) return
  await game.value?.requestLock()
}

function askNewWorld() {
  screen.value = 'confirm'
  game.value?.releaseCursor()
}

async function newWorld() {
  screen.value = 'main'
  await game.value?.newWorld()
}
</script>

<template>
  <GameCanvas ref="game" @ask-new-world="askNewWorld" @canvas-click="resumeFromLockLost" />
  <WebGpuRequired v-if="state.noWebGpu !== undefined" :reason="state.noWebGpu" />
  <Hud
    :fps="state.fps"
    :stats="state.stats"
    :time="state.time"
    :block="block"
    :player="state.player"
    :notice="state.notice"
    :show-debug="settings.showDebug"
  />
  <Hotbar v-if="state.locked && state.ready" :slot-index="slot" />
  <Inventory v-if="state.inventoryOpen && state.ready" />
  <PauseMenu
    v-if="!state.locked && !state.inventoryOpen && !state.lockLost && !BENCH"
    v-model:screen="screen"
    :settings="settings"
    :limits="limits"
    :started="started"
    :busy="state.busy"
    :lock-failed="lockFailed"
    :newer-save="state.newerSave"
    :has-error="!!state.error"
    @update:settings="update"
    @play="play"
    @new-world="newWorld"
  />
  <div v-if="state.lockLost && !state.locked && state.ready" class="click-hint">
    Кликните, чтобы продолжить
  </div>
</template>

<style scoped>
.click-hint {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 48px;
  z-index: 20;
  text-align: center;
  color: #fff;
  font-size: 14px;
  line-height: 1.4;
  text-shadow: 1px 1px 0 #000;
  pointer-events: none;
}
</style>
