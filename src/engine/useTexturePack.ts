import { parseTexturePack, type Engine, type PackInfo } from '@engine'
import { createIndexedDbPackStore, createMemoryPackStore, type PackStore } from './packStore'
import { createPackManager, type PackDescription, type PackTarget } from './texturePack'

function describe(info: PackInfo): PackDescription {
  const description = {
    description: info.description,
    icon: info.icon,
    tileSize: info.tileSize,
  }
  info.free()
  return description
}

export const engineTarget = (engine: Engine): PackTarget => ({
  loadTexturePack: (bytes) => describe(engine.loadTexturePack(bytes)),
  resetTexturePack: () => engine.resetTexturePack(),
})

function packStore(): PackStore {
  return typeof indexedDB === 'undefined' ? createMemoryPackStore() : createIndexedDbPackStore()
}

export const texturePacks = createPackManager({
  store: packStore(),
  parse: (bytes) => describe(parseTexturePack(bytes)),
  iconUrl: (icon) => URL.createObjectURL(new Blob([icon as BlobPart], { type: 'image/png' })),
  revokeIconUrl: (url) => URL.revokeObjectURL(url),
})

void texturePacks.restore()

export const useTexturePack = () => texturePacks
