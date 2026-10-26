<script setup lang="ts">
import { computed } from 'vue'
import { API_LABELS, BLOCK_LABELS, type BlockName, type GraphicsApi } from '../settings'

const props = defineProps<{
  hidden: boolean
  backend?: GraphicsApi
  renderDistance?: number
  block?: BlockName
}>()

const controls = computed(() => [
  { keys: ['Мышь'], action: 'осмотр' },
  { keys: ['Клик'], action: 'захватить курсор' },
  { keys: ['Esc'], action: 'отпустить курсор' },
  { keys: ['W', 'A', 'S', 'D'], action: 'движение' },
  { keys: ['Space'], action: 'прыжок / вверх' },
  { keys: ['Space ×2'], action: 'полёт / ходьба' },
  { keys: ['Shift'], action: 'вниз (в полёте)' },
  { keys: ['ЛКМ'], action: 'сломать' },
  { keys: ['ПКМ'], action: 'поставить' },
  { keys: ['1–3'], action: props.block ? `блок: ${BLOCK_LABELS[props.block]}` : 'блок' },
  {
    keys: ['G'],
    action: props.backend ? `графический API: ${API_LABELS[props.backend]}` : 'графический API',
  },
  {
    keys: ['[', ']'],
    action: props.renderDistance ? `дальность: ${props.renderDistance}` : 'дальность',
  },
  { keys: ['H'], action: 'скрыть панель' },
])
</script>

<template>
  <div v-if="hidden" class="panel collapsed"><kbd>H</kbd> — управление</div>
  <dl v-else class="panel">
    <template v-for="row in controls" :key="row.action">
      <dt>
        <kbd v-for="key in row.keys" :key="key">{{ key }}</kbd>
      </dt>
      <dd>{{ row.action }}</dd>
    </template>
  </dl>
</template>

<style scoped>
.panel {
  position: fixed;
  top: 8px;
  right: 8px;
  margin: 0;
  padding: 8px 10px;
  display: grid;
  grid-template-columns: auto auto;
  gap: 4px 10px;
  align-items: center;
  color: #fff;
  font-size: 13px;
  background: rgba(0, 0, 0, 0.45);
  border-radius: 6px;
  pointer-events: none;
}
/* На узком экране панель наезжает на HUD в левом углу. */
@media (max-width: 720px) {
  .panel {
    top: auto;
    bottom: 8px;
  }
}
.collapsed {
  display: block;
  opacity: 0.7;
}
dt {
  display: flex;
  gap: 2px;
  justify-content: flex-end;
}
dd {
  margin: 0;
}
kbd {
  min-width: 1.4em;
  padding: 0 4px;
  font: inherit;
  text-align: center;
  border: 1px solid rgba(255, 255, 255, 0.5);
  border-radius: 3px;
}
</style>
