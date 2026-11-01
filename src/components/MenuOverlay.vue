<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { BENCH } from '../bench/config'
import { pauseMenuEscAction } from '../input/lock-policy'
import type { Settings, SettingsLimits } from '../settings/settings'
import { useWorlds, type CreateWorldForm } from '../world/useWorlds'
import ControlsScreen from './menu/ControlsScreen.vue'
import GraphicsScreen from './menu/GraphicsScreen.vue'
import CreateWorldScreen from './menu/CreateWorldScreen.vue'
import DeleteWorldScreen from './menu/DeleteWorldScreen.vue'
import PauseScreen from './menu/PauseScreen.vue'
import SettingsScreen from './menu/SettingsScreen.vue'
import TitleScreen from './menu/TitleScreen.vue'
import WorldListScreen from './menu/WorldListScreen.vue'
import { TITLES, backTarget, type MenuScreen } from './menu/screens'

const VERSION = __APP_COMMIT__ ? `v${__APP_VERSION__} · ${__APP_COMMIT__}` : `v${__APP_VERSION__}`

const props = defineProps<{
  screen: MenuScreen
  inWorld: boolean
  settings: Settings
  limits: SettingsLimits
  started: boolean
  busy: boolean
  exiting: boolean
  lockFailed: boolean
  newerSave: boolean
  hasError: boolean
}>()

const emit = defineEmits<{
  'update:screen': [screen: MenuScreen]
  'update:settings': [patch: Partial<Settings>]
  play: []
  exit: []
  enter: [id: string]
}>()

const { worlds, selectedId, create, remove } = useWorlds()
const working = ref(false)
const selectedName = computed(() => worlds.value.find((w) => w.id === selectedId.value)?.name ?? '')
const isTitle = computed(() => !props.inWorld && props.screen === 'main')

const go = (screen: MenuScreen) => emit('update:screen', screen)
const back = () => go(backTarget(props.screen, props.inWorld))

async function guarded(action: () => Promise<void>) {
  working.value = true
  try {
    await action()
  } finally {
    working.value = false
  }
}

const createWorld = (form: CreateWorldForm) =>
  guarded(async () => emit('enter', (await create(form)).id))

const removeWorld = () =>
  guarded(async () => {
    if (selectedId.value) await remove(selectedId.value)
    go('worlds')
  })

function onKeyDown(e: KeyboardEvent) {
  if (e.code !== 'Escape' || e.repeat) return
  const action = pauseMenuEscAction({
    inWorld: props.inWorld,
    started: props.started,
    busy: props.busy,
    hasError: props.hasError,
    bench: BENCH,
    onMainScreen: props.screen === 'main',
  })
  if (action === 'back') back()
  else if (action === 'resume') emit('play')
}

onMounted(() => window.addEventListener('keydown', onKeyDown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeyDown))
</script>

<template>
  <div class="overlay px-menu" :class="inWorld ? 'dim' : 'px-menu-bg'">
    <template v-if="isTitle">
      <TitleScreen @go="go" />
      <p class="version">{{ VERSION }}</p>
    </template>
    <div v-else class="panel px-frame" role="dialog" aria-modal="true" :aria-label="TITLES[screen]">
      <h1>{{ screen === 'main' ? 'Пауза' : TITLES[screen] }}</h1>

      <PauseScreen
        v-if="screen === 'main'"
        :busy="busy"
        :exiting="exiting"
        :lock-failed="lockFailed"
        :newer-save="newerSave"
        :has-error="hasError"
        @go="go"
        @play="emit('play')"
        @exit="emit('exit')"
      />
      <WorldListScreen
        v-else-if="screen === 'worlds'"
        @play="emit('enter', $event)"
        @create="go('create')"
        @remove="go('delete')"
        @back="back"
      />
      <CreateWorldScreen
        v-else-if="screen === 'create'"
        :busy="working"
        @submit="createWorld"
        @cancel="back"
      />
      <DeleteWorldScreen
        v-else-if="screen === 'delete'"
        :name="selectedName"
        :busy="working"
        @confirm="removeWorld"
        @cancel="back"
      />
      <SettingsScreen
        v-else-if="screen === 'settings'"
        :settings="settings"
        :limits="limits"
        :show-controls="!inWorld"
        @update:settings="emit('update:settings', $event)"
        @controls="go('controls')"
        @graphics="go('graphics')"
        @back="back"
      />
      <GraphicsScreen
        v-else-if="screen === 'graphics'"
        :settings="settings"
        :limits="limits"
        @update:settings="emit('update:settings', $event)"
        @back="back"
      />
      <ControlsScreen v-else @back="back" />
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 10;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 16px;
  overflow: auto;
}
.dim {
  background: rgb(0 0 0 / 0.6);
}
.panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  box-sizing: border-box;
  width: min(624px, 100%);
  margin: auto;
  padding: 20px;
}
h1 {
  margin: 0 0 6px;
  font-size: var(--px-fs);
  font-weight: 700;
  text-align: center;
}
.version {
  position: fixed;
  bottom: 12px;
  left: 0;
  right: 0;
  margin: 0;
  font-size: var(--px-fs-s);
  text-align: center;
  color: var(--px-text-dim);
}
</style>
