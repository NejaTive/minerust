<script setup lang="ts">
defineProps<{ fps: number }>()
</script>

<template>
  <div class="hud">MineRust · {{ fps }} FPS</div>
</template>

<style scoped>
.hud {
  position: fixed;
  top: 8px;
  left: 8px;
  color: #fff;
  font-size: 14px;
  text-shadow: 1px 1px 0 #000;
  pointer-events: none;
}
</style>
