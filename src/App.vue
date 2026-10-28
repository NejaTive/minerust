<script setup lang="ts">
import { ref, useTemplateRef, watch } from 'vue'
import GameCanvas from './components/GameCanvas.vue'
import Hotbar from './components/Hotbar.vue'
import Hud from './components/Hud.vue'
import PauseMenu from './components/PauseMenu.vue'
import type { MenuScreen } from './components/menu/screens'
import WebGpuRequired from './components/WebGpuRequired.vue'
import { BENCH } from './bench/config'
import { useGameState } from './engine/state'
import { useSettings } from './settings/useSettings'

const state = useGameState()
const { settings, limits, slot, block, update } = useSettings()

const game = useTemplateRef('game')
// Меню видно, пока курсор не захвачен: это и есть пауза.
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

// N в игре: курсор отпускаем, чтобы подтверждение было видно и доступно мышью.
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
  <GameCanvas ref="game" @ask-new-world="askNewWorld" />
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
  <PauseMenu
    v-if="!state.locked && !BENCH"
    v-model:screen="screen"
    :settings="settings"
    :limits="limits"
    :started="started"
    :busy="state.busy"
    :lock-failed="lockFailed"
    :newer-save="state.newerSave"
    @update:settings="update"
    @play="play"
    @new-world="newWorld"
  />
</template>
