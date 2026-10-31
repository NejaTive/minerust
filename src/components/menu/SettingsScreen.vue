<script setup lang="ts">
import type { Settings, SettingsLimits } from '../../settings/settings'

defineProps<{ settings: Settings; limits: SettingsLimits; showControls: boolean }>()

const emit = defineEmits<{
  'update:settings': [patch: Partial<Settings>]
  controls: []
  back: []
}>()

const patch = (value: Partial<Settings>) => emit('update:settings', value)

function numberOf(e: Event): number {
  return (e.target as HTMLInputElement).valueAsNumber
}

function checkedOf(e: Event): boolean {
  return (e.target as HTMLInputElement).checked
}
</script>

<template>
  <div class="rows">
    <label class="row">
      <span class="name">Дальность прорисовки <kbd>[</kbd><kbd>]</kbd></span>
      <span class="value">{{ settings.renderDistance }}</span>
      <input
        type="range"
        :min="limits.renderDistanceMin"
        :max="limits.renderDistanceMax"
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

    <label class="row toggle">
      <span class="name">Облака</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.clouds"
        @change="patch({ clouds: checkedOf($event) })"
      />
    </label>

    <label class="row toggle">
      <span class="name">Погода</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.weather"
        @change="patch({ weather: checkedOf($event) })"
      />
    </label>

    <label class="row toggle">
      <span class="name">Автоступенька</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.autoStep"
        @change="patch({ autoStep: checkedOf($event) })"
      />
    </label>

    <label class="row">
      <span class="name">Чувствительность мыши</span>
      <span class="value">×{{ settings.mouseSensitivity.toFixed(2) }}</span>
      <input
        type="range"
        :min="limits.mouseSensitivityMin"
        :max="limits.mouseSensitivityMax"
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
        :min="limits.fovMin"
        :max="limits.fovMax"
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
  <button v-if="showControls" @click="emit('controls')">Управление</button>
  <button @click="emit('back')">Назад</button>
</template>

<style scoped>
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
}
.toggle input {
  margin: 0;
}
.value {
  font-variant-numeric: tabular-nums;
  color: var(--px-text-dim);
}
.name kbd {
  margin-left: 4px;
  color: var(--px-text-dim);
}
</style>
