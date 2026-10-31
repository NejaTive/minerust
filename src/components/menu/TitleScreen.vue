<script setup lang="ts">
import type { MenuScreen } from './screens'

const emit = defineEmits<{ go: [screen: MenuScreen] }>()
</script>

<template>
  <h1 class="logo" aria-label="MineRust">
    <span class="mine">Mine</span><span class="rust">Rust</span>
  </h1>
  <div class="buttons">
    <button class="primary" @click="emit('go', 'worlds')">Одиночная игра</button>
    <button disabled title="Скоро">Сетевая игра</button>
    <p class="hint soon">скоро</p>
    <button @click="emit('go', 'settings')">Настройки</button>
  </div>
</template>

<style scoped>
.logo {
  margin: 0 0 40px;
  font-size: calc(var(--px-fs-s) * 6);
  font-weight: 700;
  line-height: 1;
  letter-spacing: 4px;
  text-align: center;
  text-shadow:
    4px 4px 0 #000,
    8px 8px 0 #1c1c1c;
}
.mine {
  color: #7bbf4c;
}
.rust {
  color: #e0782d;
}
.buttons {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: min(520px, 100%);
}
.soon {
  margin-top: -6px;
  font-size: var(--px-fs-s);
  text-align: center;
}
@media (max-width: 520px) {
  .logo {
    font-size: calc(var(--px-fs-s) * 3);
  }
}
</style>
