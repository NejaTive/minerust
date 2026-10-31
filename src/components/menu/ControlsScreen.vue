<script setup lang="ts">
const emit = defineEmits<{ back: [] }>()

const CONTROLS = [
  { keys: ['Esc'], action: 'пауза и меню' },
  { keys: ['W', 'A', 'S', 'D'], action: 'движение' },
  { keys: ['Мышь'], action: 'осмотр' },
  { keys: ['Space'], action: 'прыжок / вверх' },
  { keys: ['Space ×2'], action: 'полёт / ходьба' },
  { keys: ['W ×2'], action: 'бег' },
  { keys: ['Shift'], action: 'вниз (в полёте)' },
  { keys: ['V'], action: 'полёт сквозь блоки (при отладке)' },
  { keys: ['ЛКМ'], action: 'сломать блок' },
  { keys: ['ПКМ'], action: 'поставить блок' },
  { keys: ['1–9', 'колесо'], action: 'слот хотбара (пусто — рука пуста)' },
  { keys: ['E'], action: 'креатив-инвентарь' },
  { keys: ['[', ']'], action: 'дальность прорисовки' },
]
</script>

<template>
  <dl class="controls">
    <template v-for="row in CONTROLS" :key="row.action">
      <dt>
        <kbd v-for="key in row.keys" :key="key">{{ key }}</kbd>
      </dt>
      <dd>{{ row.action }}</dd>
    </template>
  </dl>
  <button @click="emit('back')">Назад</button>
</template>

<style scoped>
.controls {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 8px 14px;
  align-items: center;
  margin: 0;
}
dt {
  display: flex;
  gap: 3px;
  justify-content: flex-end;
}
dd {
  margin: 0;
  color: var(--px-text);
}
</style>
