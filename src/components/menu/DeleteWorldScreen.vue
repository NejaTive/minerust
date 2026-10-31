<script setup lang="ts">
defineProps<{ name: string; busy: boolean }>()
const emit = defineEmits<{ confirm: []; cancel: [] }>()
</script>

<template>
  <p>Удалить мир «{{ name }}»? Это нельзя отменить.</p>
  <button class="danger" :disabled="busy" @click="emit('confirm')">Удалить</button>
  <button @click="emit('cancel')">Отмена</button>
</template>
