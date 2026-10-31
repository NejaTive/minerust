import { computed, ref } from 'vue'

export function createWorldSession() {
  const activeId = ref<string | null>(null)
  return {
    activeId,
    inWorld: computed(() => activeId.value !== null),
    enter(id: string) {
      activeId.value = id
    },
    exit() {
      activeId.value = null
    },
  }
}

export const worldSession = createWorldSession()
