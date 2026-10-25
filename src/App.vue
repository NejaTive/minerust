<script setup lang="ts">
import { ref } from 'vue'
import GameCanvas from './components/GameCanvas.vue'
import Hud from './components/Hud.vue'

const fps = ref(0)
</script>

<template>
  <GameCanvas @fps="fps = $event" />
  <Hud :fps="fps" />
</template>
