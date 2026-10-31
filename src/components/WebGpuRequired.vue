<script setup lang="ts">
defineProps<{
  reason: string
}>()
</script>

<template>
  <div class="screen" role="alert">
    <div class="card">
      <h1>Нужен браузер с WebGPU</h1>
      <p>MineRust рисует мир через WebGPU.</p>
      <p>Подойдут актуальные Chrome и Edge, Safari 26 и новее, Firefox с включённым WebGPU.</p>
      <p>Обновите браузер или включите аппаратное ускорение в его настройках.</p>
      <p v-if="reason" class="reason">{{ reason }}</p>
    </div>
  </div>
</template>

<style scoped>
.screen {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: grid;
  place-items: center;
  padding: 16px;
  color: #fff;
  background: #1b1b1b;
}
.card {
  max-width: 480px;
  line-height: 1.5;
}
h1 {
  margin: 0 0 12px;
  font-size: var(--px-fs);
}
p {
  margin: 0 0 10px;
}
.reason {
  color: var(--px-text-dim);
  font-size: var(--px-fs-s);
  word-break: break-word;
}
</style>
