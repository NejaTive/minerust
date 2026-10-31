<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { MODE_LABELS, WORLD_TYPE_LABELS } from '../../world/kinds'
import { useWorlds } from '../../world/useWorlds'

const emit = defineEmits<{ play: [id: string]; create: []; remove: []; back: [] }>()

const { worlds, selectedId, persistent, refresh } = useWorlds()
const selected = computed(() => worlds.value.find((w) => w.id === selectedId.value))

onMounted(refresh)

const played = (ts: number) =>
  new Date(ts).toLocaleString('ru-RU', { dateStyle: 'short', timeStyle: 'short' })
</script>

<template>
  <p v-if="!persistent" class="warn">
    Хранилище недоступно — миры не сохранятся после перезагрузки
  </p>
  <ul v-if="worlds.length" class="list" role="listbox" aria-label="Миры">
    <li v-for="world in worlds" :key="world.id">
      <button
        class="row"
        :class="{ selected: world.id === selectedId }"
        role="option"
        :aria-selected="world.id === selectedId"
        @click="selectedId = world.id"
        @dblclick="emit('play', world.id)"
      >
        <span class="name">{{ world.name }}</span>
        <span class="info">
          {{ MODE_LABELS[world.mode] ?? world.mode }} ·
          {{ WORLD_TYPE_LABELS[world.worldType] ?? world.worldType }}
        </span>
        <span class="info">Последняя игра: {{ played(world.lastPlayed) }}</span>
      </button>
    </li>
  </ul>
  <p v-else class="hint empty">Миров пока нет. Создайте первый!</p>
  <div class="actions">
    <button class="primary" :disabled="!selected" @click="selected && emit('play', selected.id)">
      Играть
    </button>
    <button @click="emit('create')">Создать мир</button>
    <button class="danger" :disabled="!selected" @click="emit('remove')">Удалить</button>
    <button @click="emit('back')">Назад</button>
  </div>
</template>

<style scoped>
.list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 288px;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}
.row {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  text-align: left;
}
.name {
  font-size: var(--px-fs);
}
.info {
  font-size: var(--px-fs-s);
  color: var(--px-text-dim);
}
.empty {
  padding: 24px 0;
  text-align: center;
}
.actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
</style>
