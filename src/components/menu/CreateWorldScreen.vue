<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  GAME_MODES,
  MODE_LABELS,
  WORLD_TYPES,
  WORLD_TYPE_LABELS,
  cycle,
  type GameMode,
  type WorldType,
} from '../../world/kinds'
import { DEFAULT_WORLD_NAME, MAX_WORLD_NAME, uniqueName } from '../../world/names'
import { useWorlds, type CreateWorldForm } from '../../world/useWorlds'

defineProps<{ busy: boolean }>()
const emit = defineEmits<{ submit: [form: CreateWorldForm]; cancel: [] }>()

const { worlds, refresh } = useWorlds()
const name = ref(DEFAULT_WORLD_NAME)
const seed = ref('')
const mode = ref<GameMode>(GAME_MODES[0])
const worldType = ref<WorldType>(WORLD_TYPES[0])

onMounted(async () => {
  await refresh()
  name.value = uniqueName(
    DEFAULT_WORLD_NAME,
    worlds.value.map((w) => w.name),
  )
})

const submit = () =>
  emit('submit', {
    name: name.value,
    seed: seed.value,
    mode: mode.value,
    worldType: worldType.value,
  })
</script>

<template>
  <form class="form" @submit.prevent="submit">
    <label class="field">
      <span>Название</span>
      <input v-model="name" type="text" :maxlength="MAX_WORLD_NAME" autocomplete="off" />
    </label>
    <button type="button" @click="mode = cycle(GAME_MODES, mode)">
      Режим игры: {{ MODE_LABELS[mode] }}
    </button>
    <button type="button" @click="worldType = cycle(WORLD_TYPES, worldType)">
      Тип мира: {{ WORLD_TYPE_LABELS[worldType] }}
    </button>
    <label class="field">
      <span>Сид</span>
      <input v-model="seed" type="text" placeholder="Случайный" autocomplete="off" />
    </label>
    <button type="submit" class="primary" :disabled="busy">Создать</button>
    <button type="button" @click="emit('cancel')">Отмена</button>
  </form>
</template>

<style scoped>
.form {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
</style>
