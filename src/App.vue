<script setup lang="ts">
import { ref, watch } from 'vue'
import GameCanvas, { type ApiInfo, type WorldStats } from './components/GameCanvas.vue'
import Hud from './components/Hud.vue'
import ControlsPanel from './components/ControlsPanel.vue'
import { loadControlsHidden, saveControlsHidden } from './settings'

const NOTICE_MS = 2500

const fps = ref(0)
const api = ref<ApiInfo>()
const stats = ref<WorldStats>()
const notice = ref<string>()
const controlsHidden = ref(loadControlsHidden())

let noticeTimer = 0

function showNotice(text: string) {
  notice.value = text
  clearTimeout(noticeTimer)
  noticeTimer = window.setTimeout(() => (notice.value = undefined), NOTICE_MS)
}

watch(controlsHidden, saveControlsHidden)
</script>

<template>
  <GameCanvas
    @fps="fps = $event"
    @stats="stats = $event"
    @api="api = $event"
    @notice="showNotice"
    @toggle-controls="controlsHidden = !controlsHidden"
  />
  <Hud :fps="fps" :api="api" :stats="stats" :notice="notice" />
  <ControlsPanel :hidden="controlsHidden" :backend="api?.backend" />
</template>
