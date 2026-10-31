<script setup lang="ts">
import type { MenuScreen } from './screens'

defineProps<{
  busy: boolean
  exiting: boolean
  lockFailed: boolean
  newerSave: boolean
  hasError: boolean
}>()

const emit = defineEmits<{ go: [screen: MenuScreen]; play: []; exit: [] }>()
</script>

<template>
  <button class="primary" :disabled="busy || exiting" @click="emit('play')">
    {{ busy ? 'Загрузка…' : 'Вернуться к игре' }}
  </button>
  <p v-if="lockFailed" class="warn">Браузер не дал захватить курсор — нажмите ещё раз</p>
  <p v-if="newerSave" class="warn">
    Сохранение из более новой версии игры: оно не загружено и не изменится
  </p>
  <button @click="emit('go', 'settings')">Настройки</button>
  <button @click="emit('go', 'controls')">Управление</button>
  <button :disabled="exiting || (busy && !hasError)" @click="emit('exit')">
    {{ exiting ? 'Сохранение…' : 'Сохранить и выйти в главное меню' }}
  </button>
</template>
