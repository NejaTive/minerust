<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue'
import GameCanvas from './components/GameCanvas.vue'
import Hotbar from './components/Hotbar.vue'
import Hud from './components/Hud.vue'
import Inventory from './components/Inventory.vue'
import MenuOverlay from './components/MenuOverlay.vue'
import type { MenuScreen } from './components/menu/screens'
import StartOverlay from './components/StartOverlay.vue'
import WebGpuRequired from './components/WebGpuRequired.vue'
import { BENCH } from './bench/config'
import { hasWebGpu } from './engine/gpu'
import { startOverlay } from './input/lock-policy'
import { useGameState } from './engine/state'
import { useSettings } from './settings/useSettings'
import { worldSession as session } from './world/session'

const BENCH_WORLD_ID = 'bench'

const state = useGameState()
const { settings, limits, slot, block, update } = useSettings()

const game = useTemplateRef('game')
const started = ref(false)
const exiting = ref(false)
const lockFailed = ref(false)
const screen = ref<MenuScreen>('main')
const pendingStart = ref(false)

const overlay = computed(() =>
  startOverlay({
    inWorld: session.inWorld.value,
    pendingStart: pendingStart.value,
    busy: state.busy,
    locked: state.locked,
    hasError: !!state.error,
  }),
)

watch(
  () => state.busy,
  (busy) => {
    if (!busy && pendingStart.value) void play()
  },
)

function onKeyDown(e: KeyboardEvent) {
  if (e.code === 'Escape' && !e.repeat && overlay.value === 'click') pendingStart.value = false
}

watch(
  () => state.locked,
  (locked) => {
    if (!locked) return
    started.value = true
    pendingStart.value = false
    lockFailed.value = false
    screen.value = 'main'
  },
)

onMounted(() => {
  window.addEventListener('keydown', onKeyDown)
  if (!hasWebGpu()) state.noWebGpu = ''
  else if (BENCH) session.enter(BENCH_WORLD_ID)
})

onBeforeUnmount(() => window.removeEventListener('keydown', onKeyDown))

async function play() {
  lockFailed.value = !(await game.value?.requestLock())
}

async function resumeFromLockLost() {
  if (!state.lockLost) return
  await game.value?.requestLock()
}

function enterWorld(id: string) {
  started.value = false
  lockFailed.value = false
  pendingStart.value = true
  screen.value = 'main'
  session.enter(id)
}

async function exitToMenu() {
  exiting.value = true
  try {
    await game.value?.save()
  } finally {
    session.exit()
    pendingStart.value = false
    screen.value = 'main'
    exiting.value = false
  }
}
</script>

<template>
  <GameCanvas
    v-if="session.activeId.value"
    :key="session.activeId.value"
    ref="game"
    :world-id="session.activeId.value"
    @canvas-click="resumeFromLockLost"
  />
  <WebGpuRequired v-if="state.noWebGpu !== undefined" :reason="state.noWebGpu" />
  <template v-if="session.inWorld.value">
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
  </template>
  <MenuOverlay
    v-if="!BENCH && overlay === 'none' && !state.locked && !state.inventoryOpen && !state.lockLost"
    v-model:screen="screen"
    :in-world="session.inWorld.value"
    :settings="settings"
    :limits="limits"
    :started="started"
    :busy="state.busy"
    :exiting="exiting"
    :lock-failed="lockFailed"
    :newer-save="state.newerSave"
    :has-error="!!state.error"
    @update:settings="update"
    @play="play"
    @exit="exitToMenu"
    @enter="enterWorld"
  />
  <StartOverlay v-if="overlay !== 'none'" :loading="overlay === 'loading'" @start="play" />
  <div v-if="state.lockLost && !state.locked && state.ready" class="click-hint px-text">
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
  font-size: var(--px-fs);
  line-height: 1.4;
  pointer-events: none;
}
</style>
