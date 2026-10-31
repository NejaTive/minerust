import { ref } from 'vue'
import { engineInfo } from '../engine/info'
import type { GameMode, WorldType } from './kinds'
import { uniqueName } from './names'
import { seedFromInput } from './seed'
import { createMemoryStorage, openStorage, type StoredWorld, type WorldsStore } from './storage'

const worlds = ref<StoredWorld[]>([])
const selectedId = ref<string | null>(null)
const persistent = ref(true)
let store: Promise<WorldsStore> | undefined

export function worldsStore(): Promise<WorldsStore> {
  store ??= openStorage().then((opened) => {
    if (opened) return opened
    persistent.value = false
    return createMemoryStorage()
  })
  return store
}

const randomSeed = () => crypto.getRandomValues(new Int32Array(1))[0]!

export interface CreateWorldForm {
  name: string
  seed: string
  mode: GameMode
  worldType: WorldType
}

export function useWorlds() {
  async function refresh() {
    worlds.value = await (await worldsStore()).list()
    if (!worlds.value.some((w) => w.id === selectedId.value)) selectedId.value = null
  }

  async function create(form: CreateWorldForm): Promise<StoredWorld> {
    await refresh()
    const created = await (
      await worldsStore()
    ).create(
      {
        name: uniqueName(
          form.name,
          worlds.value.map((w) => w.name),
        ),
        seed: seedFromInput(form.seed, randomSeed),
        mode: form.mode,
        worldType: form.worldType,
      },
      engineInfo().formatVersion,
    )
    await refresh()
    return created
  }

  async function remove(id: string) {
    await (await worldsStore()).remove(id)
    await refresh()
  }

  return { worlds, selectedId, persistent, refresh, create, remove }
}
