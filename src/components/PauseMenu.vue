<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { BENCH } from '../bench/config'
import { pauseMenuEscAction } from '../input/lock-policy'
import type { Settings, SettingsLimits } from '../settings/settings'
import ConfirmScreen from './menu/ConfirmScreen.vue'
import ControlsScreen from './menu/ControlsScreen.vue'
import MainScreen from './menu/MainScreen.vue'
import SettingsScreen from './menu/SettingsScreen.vue'
import { TITLES, type MenuScreen } from './menu/screens'
import './menu/menu.css'

const props = defineProps<{
  screen: MenuScreen
  settings: Settings
  limits: SettingsLimits
  /** Игра уже шла: вместо «Играть» — «Продолжить». */
  started: boolean
  /** Движок грузится или пересоздаётся: играть и менять мир пока нельзя. */
  busy: boolean
  /** Браузер отказал в захвате курсора. */
  lockFailed: boolean
  /** В базе мир более новой версии игры: он не загружен, «Новый мир» его сотрёт. */
  newerSave: boolean
  /** Движок упал: Esc не должен пытаться продолжить. */
  hasError: boolean
}>()

const emit = defineEmits<{
  'update:screen': [screen: MenuScreen]
  'update:settings': [patch: Partial<Settings>]
  play: []
  'new-world': []
}>()

const go = (screen: MenuScreen) => emit('update:screen', screen)

// Из игры Esc сюда не доходит (браузер снимает lock). В меню: подменю → назад,
// главный после старта → «Продолжить» (отказ Chrome ~1 с — меню остаётся).
function onKeyDown(e: KeyboardEvent) {
  if (e.code !== 'Escape' || e.repeat) return
  const action = pauseMenuEscAction({
    started: props.started,
    busy: props.busy,
    hasError: props.hasError,
    bench: BENCH,
    onMainScreen: props.screen === 'main',
  })
  if (action === 'back-to-main') go('main')
  else if (action === 'resume') emit('play')
}

onMounted(() => window.addEventListener('keydown', onKeyDown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeyDown))
</script>

<template>
  <div class="overlay">
    <div class="panel" role="dialog" aria-modal="true" :aria-label="TITLES[screen]">
      <h1>{{ TITLES[screen] }}</h1>

      <MainScreen
        v-if="screen === 'main'"
        :started="started"
        :busy="busy"
        :lock-failed="lockFailed"
        :newer-save="newerSave"
        @go="go"
        @play="emit('play')"
      />
      <SettingsScreen
        v-else-if="screen === 'settings'"
        :settings="settings"
        :limits="limits"
        @update:settings="emit('update:settings', $event)"
        @back="go('main')"
      />
      <ControlsScreen v-else-if="screen === 'controls'" @back="go('main')" />
      <ConfirmScreen v-else :busy="busy" @back="go('main')" @new-world="emit('new-world')" />
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 10;
  display: grid;
  place-items: center;
  padding: 16px;
  overflow: auto;
  color: #f2f4f7;
  font-size: 15px;
  background: rgba(8, 10, 14, 0.62);
}
.panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  box-sizing: border-box;
  width: min(420px, 100%);
  padding: 20px;
  background: #1b1f26;
  border: 1px solid #343b47;
  border-radius: 12px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
}
h1 {
  margin: 0 0 6px;
  font-size: 22px;
  font-weight: 600;
  text-align: center;
}
</style>
