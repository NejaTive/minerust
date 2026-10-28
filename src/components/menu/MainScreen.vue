<script setup lang="ts">
import type { MenuScreen } from './screens'

defineProps<{
  /** Игра уже шла: вместо «Играть» — «Продолжить». */
  started: boolean
  /** Движок грузится или пересоздаётся: играть и менять мир пока нельзя. */
  busy: boolean
  /** Браузер отказал в захвате курсора. */
  lockFailed: boolean
  /** В базе мир более новой версии игры: он не загружен, «Новый мир» его сотрёт. */
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
