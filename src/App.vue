<script setup lang="ts">
import { reactive, ref, useTemplateRef, watch } from 'vue'
import GameCanvas, {
  type ApiInfo,
  type PlayerInfo,
  type WorldStats,
} from './components/GameCanvas.vue'
import Hud from './components/Hud.vue'
import PauseMenu, { type MenuScreen } from './components/PauseMenu.vue'
import { type BlockName, type Settings, loadSettings, saveSettings } from './settings'

const NOTICE_MS = 2500

const settings = reactive<Settings>(loadSettings())
const fps = ref(0)
const api = ref<ApiInfo>()
const stats = ref<WorldStats>()
const notice = ref<string>()
const block = ref<BlockName>()
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

watch(settings, saveSettings)

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
    :settings="settings"
    @fps="fps = $event"
    @stats="stats = $event"
    @api="api = $event"
    @notice="showNotice"
    @render-distance="settings.renderDistance = $event"
    @block="block = $event"
    @player="player = $event"
    @lock="onLock"
    @busy="busy = $event"
    @ask-new-world="askNewWorld"
  />
  <Hud
    :fps="fps"
    :api="api"
    :stats="stats"
    :block="block"
    :player="player"
    :notice="notice"
    :show-debug="settings.showDebug"
  />
  <PauseMenu
    v-if="!locked"
    v-model:screen="screen"
    :settings="settings"
    :api="api"
    :started="started"
    :busy="busy"
    :lock-failed="lockFailed"
    @update:settings="updateSettings"
    @play="play"
    @set-api="game?.switchApi($event)"
    @new-world="newWorld"
  />
</template>
