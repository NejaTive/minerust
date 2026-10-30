<script setup lang="ts">
import type { MenuScreen } from './screens'

defineProps<{
  started: boolean
  busy: boolean
  lockFailed: boolean
  newerSave: boolean
}>()

const emit = defineEmits<{ go: [screen: MenuScreen]; play: [] }>()

const VERSION = __APP_COMMIT__ ? `v${__APP_VERSION__} · ${__APP_COMMIT__}` : `v${__APP_VERSION__}`
</script>

<template>
  <button class="primary" :disabled="busy" @click="emit('play')">
    {{ busy ? 'Загрузка…' : started ? 'Продолжить' : 'Играть' }}
  </button>
  <p v-if="lockFailed" class="warn">Браузер не дал захватить курсор — нажмите ещё раз</p>
  <p v-if="newerSave" class="warn">
    Сохранение из более новой версии игры: оно не загружено и не изменится, пока вы не начнёте новый
    мир
  </p>
  <button :disabled="busy" @click="emit('go', 'confirm')">Новый мир</button>
  <button @click="emit('go', 'settings')">Настройки</button>
  <button @click="emit('go', 'controls')">Управление</button>
  <p class="version">{{ VERSION }}</p>
</template>

<style scoped>
.version {
  margin: 4px 0 0;
  font-size: 12px;
  color: #7b8494;
  text-align: center;
}
</style>
