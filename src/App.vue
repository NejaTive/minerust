<script setup lang="ts">
import { computed, reactive, ref, useTemplateRef, watch } from 'vue'
import GameCanvas, {
  type PlayerInfo,
  type TimeInfo,
  type WorldStats,
} from './components/GameCanvas.vue'
import Hotbar from './components/Hotbar.vue'
import Hud from './components/Hud.vue'
import WebGpuRequired from './components/WebGpuRequired.vue'
import PauseMenu, { type MenuScreen } from './components/PauseMenu.vue'
import { BENCH, BENCH_DEBUG, BENCH_RENDER_DISTANCE } from './bench'
import {
  HOTBAR,
  type Settings,
  loadHotbarSlot,
  loadSettings,
  saveHotbarSlot,
  saveSettings,
} from './settings'

const NOTICE_MS = 2500

const settings = reactive<Settings>(loadSettings())
// Режим замера: фиксированные настройки, поверх сохранённых, и в localStorage они не попадают.
if (BENCH) {
  Object.assign(settings, {
    renderDistance: BENCH_RENDER_DISTANCE,
    ambientOcclusion: true,
    showDebug: BENCH_DEBUG,
  })
}
const fps = ref(0)
// Движок создан: до этого хотбар не показываем.
const ready = ref(false)
// WebGPU нет; пустая строка — причина неизвестна (нет `navigator.gpu`).
const noWebGpu = ref<string>()
const stats = ref<WorldStats>()
const time = ref<TimeInfo>()
const notice = ref<string>()
const slot = ref(loadHotbarSlot())
const block = computed(() => HOTBAR[slot.value] ?? null)
const player = ref<PlayerInfo>()

const game = useTemplateRef('game')
// Меню видно, пока курсор не захвачен: это и есть пауза.
const locked = ref(false)
const busy = ref(true)
const started = ref(false)
const lockFailed = ref(false)
const screen = ref<MenuScreen>('main')

let noticeTimer = 0

function showNotice(text: string) {
  notice.value = text
  clearTimeout(noticeTimer)
  noticeTimer = window.setTimeout(() => (notice.value = undefined), NOTICE_MS)
}

watch(settings, (value) => {
  if (!BENCH) saveSettings(value)
})
watch(slot, saveHotbarSlot)

function updateSettings(patch: Partial<Settings>) {
  Object.assign(settings, patch)
}

function onLock(value: boolean) {
  locked.value = value
  if (value) {
    started.value = true
    lockFailed.value = false
    screen.value = 'main'
  }
}

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
  <GameCanvas
    ref="game"
    v-model:slot="slot"
    :settings="settings"
    @fps="fps = $event"
    @stats="stats = $event"
    @time="time = $event"
    @ready="ready = true"
    @no-webgpu="noWebGpu = $event ?? ''"
    @notice="showNotice"
    @render-distance="settings.renderDistance = $event"
    @player="player = $event"
    @lock="onLock"
    @busy="busy = $event"
    @ask-new-world="askNewWorld"
  />
  <WebGpuRequired v-if="noWebGpu !== undefined" :reason="noWebGpu" />
  <Hud
    :fps="fps"
    :stats="stats"
    :time="time"
    :block="block"
    :player="player"
    :notice="notice"
    :show-debug="settings.showDebug"
  />
  <Hotbar v-if="locked && ready" :slot-index="slot" />
  <PauseMenu
    v-if="!locked && !BENCH"
    v-model:screen="screen"
    :settings="settings"
    :started="started"
    :busy="busy"
    :lock-failed="lockFailed"
    @update:settings="updateSettings"
    @play="play"
    @new-world="newWorld"
  />
</template>
