<script setup lang="ts">
defineProps<{ loading: boolean }>()
const emit = defineEmits<{ start: [] }>()
</script>

<template>
  <div
    class="start px-menu px-menu-bg"
    :class="{ ready: !loading }"
    @click="!loading && emit('start')"
  >
    <p>{{ loading ? 'Загрузка мира…' : 'Кликните, чтобы начать' }}</p>
  </div>
</template>

<style scoped>
.start {
  position: fixed;
  inset: 0;
  z-index: 10;
  display: grid;
  place-items: center;
  font-size: var(--px-fs);
}
.ready {
  cursor: pointer;
}
</style>
