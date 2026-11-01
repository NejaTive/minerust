<script setup lang="ts">
import type { Settings, SettingsLimits } from '../../settings/settings'

defineProps<{ settings: Settings; limits: SettingsLimits }>()

const emit = defineEmits<{
  'update:settings': [patch: Partial<Settings>]
  back: []
}>()

const patch = (value: Partial<Settings>) => emit('update:settings', value)

function numberOf(e: Event): number {
  return (e.target as HTMLInputElement).valueAsNumber
}

function qualityOf(e: Event): boolean {
  return (e.target as HTMLSelectElement).value === 'high'
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
      <span class="name">Цветной свет</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.coloredLight"
        @change="patch({ coloredLight: checkedOf($event) })"
      />
    </label>

    <label class="row toggle">
      <span class="name">Отражения на воде</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.waterReflections"
        @change="patch({ waterReflections: checkedOf($event) })"
      />
    </label>

    <label class="row toggle">
      <span class="name">Наземный туман</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.groundFog"
        @change="patch({ groundFog: checkedOf($event) })"
      />
    </label>

    <label class="row toggle">
      <span class="name">Колыхание растений</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.plantSway"
        @change="patch({ plantSway: checkedOf($event) })"
      />
    </label>

    <label class="row toggle">
      <span class="name">HDR и тонмаппинг</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.hdr"
        @change="patch({ hdr: checkedOf($event) })"
      />
    </label>

    <label class="row toggle" :class="{ disabled: !settings.hdr }">
      <span class="name">Свечение</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.hdr && settings.bloom"
        :disabled="!settings.hdr"
        @change="patch({ bloom: checkedOf($event) })"
      />
    </label>

    <label class="row toggle" :class="{ disabled: !settings.hdr }">
      <span class="name">Глубина воды</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.hdr && settings.waterDepth"
        :disabled="!settings.hdr"
        @change="patch({ waterDepth: checkedOf($event) })"
      />
    </label>

    <label class="row toggle" :class="{ disabled: !settings.hdr || !settings.waterReflections }">
      <span class="name">Экранные отражения</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.hdr && settings.waterReflections && settings.ssr"
        :disabled="!settings.hdr || !settings.waterReflections"
        @change="patch({ ssr: checkedOf($event) })"
      />
    </label>

    <label class="row toggle" :class="{ disabled: !settings.hdr }">
      <span class="name">Солнечные лучи</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.hdr && settings.sunRays"
        :disabled="!settings.hdr"
        @change="patch({ sunRays: checkedOf($event) })"
      />
    </label>

    <label class="row toggle" :class="{ disabled: !settings.hdr }">
      <span class="name">Автоэкспозиция</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.hdr && settings.autoExposure"
        :disabled="!settings.hdr"
        @change="patch({ autoExposure: checkedOf($event) })"
      />
    </label>

    <label class="row toggle">
      <span class="name">Тени</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.shadows"
        @change="patch({ shadows: checkedOf($event) })"
      />
    </label>

    <label class="row toggle" :class="{ disabled: !settings.shadows }">
      <span class="name">Качество теней</span>
      <select
        :value="settings.shadowQualityHigh ? 'high' : 'low'"
        :disabled="!settings.shadows"
        @change="patch({ shadowQualityHigh: qualityOf($event) })"
      >
        <option value="low">Низкое</option>
        <option value="high">Высокое</option>
      </select>
    </label>

    <label class="row toggle">
      <span class="name">Тени облаков</span>
      <input
        type="checkbox"
        role="switch"
        :checked="settings.cloudShadows"
        @change="patch({ cloudShadows: checkedOf($event) })"
      />
    </label>
  </div>
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
.disabled {
  opacity: 0.5;
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
