<script setup lang="ts">
import { computed, nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { TabCycler } from '../console/complete'
import { classifyConsoleKey } from '../console/keys'
import {
  closeConsole,
  commandHistory,
  commandUsage,
  completeCommand,
  consoleInput,
  submitCommand,
} from '../engine/console'

const line = consoleInput

const input = useTemplateRef<HTMLInputElement>('input')
const tab = new TabCycler()

const options = computed(() => completeCommand(line.value).options)
const usage = computed(() => commandUsage(line.value))

function setLine(value: string) {
  line.value = value
  void nextTick(() => input.value?.setSelectionRange(value.length, value.length))
}

function onKeyDown(e: KeyboardEvent) {
  switch (classifyConsoleKey(e.code)) {
    case 'submit':
      e.preventDefault()
      submitCommand(line.value)
      closeConsole()
      return
    case 'complete': {
      e.preventDefault()
      const completed = tab.next(line.value, completeCommand)
      if (completed !== undefined) setLine(completed)
      return
    }
    case 'history-up': {
      e.preventDefault()
      const previous = commandHistory.previous(line.value)
      if (previous !== undefined) setLine(previous)
      return
    }
    case 'history-down': {
      e.preventDefault()
      const next = commandHistory.next()
      if (next !== undefined) setLine(next)
      return
    }
    default:
      return
  }
}

function onInput() {
  tab.reset()
  commandHistory.reset()
}

watch(line, (value) => {
  input.value?.focus()
  void nextTick(() => input.value?.setSelectionRange(value.length, value.length))
})

onMounted(() => {
  commandHistory.reset()
  input.value?.focus()
})
</script>

<template>
  <div class="console" role="dialog" aria-label="Командная строка">
    <div v-if="options.length" class="options px-text">{{ options.join('  ') }}</div>
    <div v-if="usage" class="usage px-text">{{ usage }}</div>
    <input
      ref="input"
      v-model="line"
      class="field px-text"
      type="text"
      spellcheck="false"
      autocomplete="off"
      @keydown="onKeyDown"
      @input="onInput"
    />
  </div>
</template>

<style scoped>
.console {
  position: fixed;
  left: 8px;
  right: 8px;
  bottom: 8px;
  z-index: 30;
  font-size: var(--px-fs-s);
  line-height: 1.4;
  text-shadow: 2px 2px 0 #000;
}
.options,
.usage {
  padding: 2px 8px;
  background: rgb(0 0 0 / 50%);
  white-space: pre-wrap;
}
.options {
  color: var(--px-green-light);
}
.usage {
  color: var(--px-text-dim);
}
.field {
  display: block;
  width: 100%;
  box-sizing: border-box;
  padding: 6px 8px;
  border: none;
  outline: none;
  background: rgb(0 0 0 / 70%);
  color: var(--px-text);
  font: inherit;
  text-shadow: inherit;
}
</style>
