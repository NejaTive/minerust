import { reactive } from 'vue'
import type { PackStore } from './packStore'

export interface PackDescription {
  description: string
  icon?: Uint8Array
  tileSize: number
}

export interface PackTarget {
  loadTexturePack(bytes: Uint8Array): PackDescription
  resetTexturePack(): void
}

export interface PackState {
  active: boolean
  name: string
  description: string
  iconUrl: string | undefined
  tileSize: number
  error: string
  busy: boolean
  revision: number
}

export const DEFAULT_TILE_SIZE = 32

const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e))

export function createPackManager(deps: {
  store: PackStore
  parse: (bytes: Uint8Array) => PackDescription
  iconUrl?: (icon: Uint8Array) => string
  revokeIconUrl?: (url: string) => void
}) {
  const state = reactive<PackState>({
    active: false,
    name: '',
    description: '',
    iconUrl: undefined,
    tileSize: DEFAULT_TILE_SIZE,
    error: '',
    busy: false,
    revision: 0,
  })
  let target: PackTarget | undefined
  let bytes: Uint8Array | undefined
  let restored: Promise<void> | undefined

  function show(name: string, info: PackDescription) {
    clearIcon()
    state.active = true
    state.name = name
    state.description = info.description
    state.tileSize = info.tileSize
    state.iconUrl = info.icon?.length ? deps.iconUrl?.(info.icon) : undefined
  }

  function clearIcon() {
    if (state.iconUrl) deps.revokeIconUrl?.(state.iconUrl)
    state.iconUrl = undefined
  }

  function showStandard() {
    clearIcon()
    bytes = undefined
    state.active = false
    state.name = ''
    state.description = ''
    state.tileSize = DEFAULT_TILE_SIZE
  }

  function apply() {
    if (!target) return
    if (bytes) show(state.name, target.loadTexturePack(bytes))
    else target.resetTexturePack()
    state.revision++
  }

  async function doRestore() {
    try {
      const stored = await deps.store.load()
      if (!stored) return
      show(stored.name, deps.parse(stored.bytes))
      bytes = stored.bytes
    } catch (e) {
      showStandard()
      state.error = errorText(e)
    }
  }

  return {
    state,
    restore: () => (restored ??= doRestore()),
    async attach(next: PackTarget | undefined) {
      await this.restore()
      target = next
      try {
        apply()
      } catch (e) {
        showStandard()
        target?.resetTexturePack()
        state.error = errorText(e)
      }
    },
    async install(name: string, data: Uint8Array): Promise<boolean> {
      await this.restore()
      state.busy = true
      state.error = ''
      try {
        const info = deps.parse(data)
        await deps.store.save({ name, bytes: data })
        bytes = data
        show(name, info)
        apply()
        return true
      } catch (e) {
        state.error = errorText(e)
        return false
      } finally {
        state.busy = false
      }
    },
    async reset() {
      await this.restore()
      state.busy = true
      state.error = ''
      try {
        await deps.store.clear()
      } catch (e) {
        state.error = errorText(e)
      } finally {
        showStandard()
        apply()
        state.busy = false
      }
    },
  }
}
