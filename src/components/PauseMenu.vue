<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import {
  API_LABELS,
  type GraphicsApi,
  MAX_FOV,
  MAX_MOUSE_SENSITIVITY,
  MAX_RENDER_DISTANCE,
  MIN_FOV,
  MIN_MOUSE_SENSITIVITY,
  MIN_RENDER_DISTANCE,
  type Settings,
} from '../settings'
import type { ApiInfo } from './GameCanvas.vue'

/** Экраны меню; новый пункт — новое значение здесь и ветка в шаблоне. */
export type MenuScreen = 'main' | 'settings' | 'controls' | 'confirm'

const props = defineProps<{
  screen: MenuScreen
  settings: Settings
  api?: ApiInfo
  /** Игра уже шла: вместо «Играть» — «Продолжить». */
  started: boolean
  /** Движок грузится или пересоздаётся: играть и менять мир пока нельзя. */
  busy: boolean
  /** Браузер отказал в захвате курсора. */
  lockFailed: boolean
}>()

const emit = defineEmits<{
  'update:screen': [screen: MenuScreen]
  'update:settings': [patch: Partial<Settings>]
  play: []
  'set-api': [api: GraphicsApi]
  'new-world': []
}>()

const APIS: GraphicsApi[] = ['webgpu', 'webgl2']

const CONTROLS = [
  { keys: ['Esc'], action: 'пауза и меню' },
  { keys: ['W', 'A', 'S', 'D'], action: 'движение' },
  { keys: ['Мышь'], action: 'осмотр' },
  { keys: ['Space'], action: 'прыжок / вверх' },
  { keys: ['Space ×2'], action: 'полёт / ходьба' },
  { keys: ['W ×2'], action: 'бег' },
  { keys: ['Shift'], action: 'вниз (в полёте)' },
  { keys: ['V'], action: 'полёт сквозь блоки (при отладке)' },
  { keys: ['ЛКМ'], action: 'сломать блок' },
  { keys: ['ПКМ'], action: 'поставить блок' },
  { keys: ['1–9', 'колесо'], action: 'слот хотбара' },
  { keys: ['G'], action: 'графический API' },
  { keys: ['N'], action: 'новый мир' },
  { keys: ['[', ']'], action: 'дальность прорисовки' },
]

const TITLES: Record<MenuScreen, string> = {
  main: 'MineRust',
  settings: 'Настройки',
  controls: 'Управление',
  confirm: 'Новый мир',
}

const go = (screen: MenuScreen) => emit('update:screen', screen)
const patch = (value: Partial<Settings>) => emit('update:settings', value)

function numberOf(e: Event): number {
  return (e.target as HTMLInputElement).valueAsNumber
}

function checkedOf(e: Event): boolean {
  return (e.target as HTMLInputElement).checked
}

// Браузер при захваченном курсоре забирает Esc себе, так что сюда он попадает только
// из меню: из подменю — назад, из главного ничего (продолжить можно лишь кликом).
function onKeyDown(e: KeyboardEvent) {
  if (e.code === 'Escape' && props.screen !== 'main') go('main')
}

onMounted(() => window.addEventListener('keydown', onKeyDown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeyDown))
</script>

<template>
  <div class="overlay">
    <div class="panel" role="dialog" aria-modal="true" :aria-label="TITLES[screen]">
      <h1>{{ TITLES[screen] }}</h1>

      <template v-if="screen === 'main'">
        <button class="primary" :disabled="busy" @click="emit('play')">
          {{ busy ? 'Загрузка…' : started ? 'Продолжить' : 'Играть' }}
        </button>
        <p v-if="lockFailed" class="warn">Браузер не дал захватить курсор — нажмите ещё раз</p>
        <button :disabled="busy" @click="go('confirm')">Новый мир</button>
        <button @click="go('settings')">Настройки</button>
        <button @click="go('controls')">Управление</button>
      </template>

      <template v-else-if="screen === 'settings'">
        <div class="rows">
          <label class="row">
            <span class="name">Дальность прорисовки <kbd>[</kbd><kbd>]</kbd></span>
            <span class="value">{{ settings.renderDistance }}</span>
            <input
              type="range"
              :min="MIN_RENDER_DISTANCE"
              :max="MAX_RENDER_DISTANCE"
              step="1"
              :value="settings.renderDistance"
              @input="patch({ renderDistance: numberOf($event) })"
            />
          </label>

          <label class="row toggle">
            <span class="name">Затенение углов</span>
            <input
              type="checkbox"
              role="switch"
              :checked="settings.ambientOcclusion"
              @change="patch({ ambientOcclusion: checkedOf($event) })"
            />
          </label>

          <div class="row">
            <span class="name">Графический API <kbd>G</kbd></span>
            <div class="segmented">
              <button
                v-for="name in APIS"
                :key="name"
                :class="{ active: api?.backend === name }"
                :aria-pressed="api?.backend === name"
                :disabled="busy || api?.backend === name"
                @click="emit('set-api', name)"
              >
                {{ API_LABELS[name] }}
              </button>
            </div>
            <span v-if="api?.fallback" class="note">WebGPU недоступен — работает WebGL2</span>
          </div>

          <label class="row">
            <span class="name">Чувствительность мыши</span>
            <span class="value">×{{ settings.mouseSensitivity.toFixed(2) }}</span>
            <input
              type="range"
              :min="MIN_MOUSE_SENSITIVITY"
              :max="MAX_MOUSE_SENSITIVITY"
              step="0.05"
              :value="settings.mouseSensitivity"
              @input="patch({ mouseSensitivity: numberOf($event) })"
            />
          </label>

          <label class="row">
            <span class="name">Угол обзора</span>
            <span class="value">{{ settings.fov }}°</span>
            <input
              type="range"
              :min="MIN_FOV"
              :max="MAX_FOV"
              step="1"
              :value="settings.fov"
              @input="patch({ fov: numberOf($event) })"
            />
          </label>

          <label class="row toggle">
            <span class="name">Отладочная информация</span>
            <input
              type="checkbox"
              role="switch"
              :checked="settings.showDebug"
              @change="patch({ showDebug: checkedOf($event) })"
            />
          </label>
        </div>
        <button @click="go('main')">Назад</button>
      </template>

      <template v-else-if="screen === 'controls'">
        <dl class="controls">
          <template v-for="row in CONTROLS" :key="row.action">
            <dt>
              <kbd v-for="key in row.keys" :key="key">{{ key }}</kbd>
            </dt>
            <dd>{{ row.action }}</dd>
          </template>
        </dl>
        <button @click="go('main')">Назад</button>
      </template>

      <template v-else>
        <p>Начать новый мир? Текущий будет удалён без возможности восстановления.</p>
        <button class="danger" :disabled="busy" @click="emit('new-world')">
          Удалить и начать новый
        </button>
        <button @click="go('main')">Отмена</button>
      </template>
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
p {
  margin: 0;
  line-height: 1.4;
}
.warn {
  color: #ffd75e;
  font-size: 14px;
}
button {
  padding: 11px 14px;
  color: inherit;
  font: inherit;
  background: #2b323d;
  border: 1px solid #3d4552;
  border-radius: 8px;
  cursor: pointer;
}
button:hover:not(:disabled) {
  background: #353d4a;
}
button:focus-visible,
input:focus-visible {
  outline: 2px solid #6aa8ff;
  outline-offset: 2px;
}
button:disabled {
  opacity: 0.55;
  cursor: default;
}
.primary {
  color: #fff;
  font-weight: 600;
  background: #2f7d4f;
  border-color: #3f9a64;
}
.primary:hover:not(:disabled) {
  background: #38905c;
}
.danger {
  color: #fff;
  background: #a33a3a;
  border-color: #c25050;
}
.danger:hover:not(:disabled) {
  background: #b84646;
}

.rows {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.row {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 6px 12px;
  align-items: center;
}
.row input[type='range'] {
  grid-column: 1 / -1;
  width: 100%;
  margin: 0;
  accent-color: #3f9a64;
}
.row .segmented,
.row .note {
  grid-column: 1 / -1;
}
.toggle input {
  width: 20px;
  height: 20px;
  margin: 0;
  accent-color: #3f9a64;
}
.value {
  font-variant-numeric: tabular-nums;
  color: #b8c0cc;
}
.note {
  color: #ffd75e;
  font-size: 13px;
}
.segmented {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
}
.segmented .active {
  color: #fff;
  background: #2f7d4f;
  border-color: #3f9a64;
  opacity: 1;
}

.controls {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 8px 14px;
  align-items: center;
  margin: 0;
}
dt {
  display: flex;
  gap: 3px;
  justify-content: flex-end;
}
dd {
  margin: 0;
  color: #d5dae2;
}
kbd {
  min-width: 1.4em;
  padding: 1px 5px;
  font: inherit;
  font-size: 13px;
  text-align: center;
  background: #12151a;
  border: 1px solid #4a5361;
  border-radius: 4px;
}
.name kbd {
  margin-left: 4px;
  color: #b8c0cc;
}
</style>
