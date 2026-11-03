<script setup lang="ts">
import { onMounted, useTemplateRef } from 'vue'
import { texturePacks } from '../../engine/useTexturePack'

const emit = defineEmits<{ back: [] }>()

const state = texturePacks.state
onMounted(() => void texturePacks.restore())

const input = useTemplateRef<HTMLInputElement>('input')

async function onPick(e: Event) {
  const el = e.target as HTMLInputElement
  const file = el.files?.[0]
  el.value = ''
  if (!file) return
  await texturePacks.install(file.name, new Uint8Array(await file.arrayBuffer()))
}
</script>

<template>
  <div class="pack">
    <img v-if="state.iconUrl" class="icon" :src="state.iconUrl" alt="" />
    <div class="info">
      <p class="name">{{ state.active ? state.name : 'Стандартный (Faithful 32x)' }}</p>
      <p v-if="state.description" class="desc">{{ state.description }}</p>
      <p class="size">{{ state.tileSize }}×{{ state.tileSize }}</p>
    </div>
  </div>
  <p v-if="state.error" class="warn" role="alert">{{ state.error }}</p>
  <input ref="input" type="file" accept=".zip,application/zip" hidden @change="onPick" />
  <button class="primary" :disabled="state.busy" @click="input?.click()">Выбрать .zip</button>
  <button :disabled="state.busy || !state.active" @click="texturePacks.reset()">Стандартный</button>
  <button @click="emit('back')">Назад</button>
</template>

<style scoped>
.pack {
  display: flex;
  gap: 12px;
  align-items: center;
}
.icon {
  width: 64px;
  height: 64px;
  image-rendering: pixelated;
}
.info p {
  margin: 0 0 4px;
  white-space: pre-line;
}
.desc,
.size {
  color: var(--px-text-dim);
}
</style>
