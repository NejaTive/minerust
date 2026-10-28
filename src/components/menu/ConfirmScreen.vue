<script setup lang="ts">
defineProps<{ busy: boolean }>()
const emit = defineEmits<{ back: []; 'new-world': [] }>()
</script>

<template>
  <p>Начать новый мир? Текущий будет удалён без возможности восстановления.</p>
  <button class="danger" :disabled="busy" @click="emit('new-world')">Удалить и начать новый</button>
  <button @click="emit('back')">Отмена</button>
</template>
