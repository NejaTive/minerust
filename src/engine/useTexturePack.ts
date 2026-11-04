import { builtinGui, parseTexturePack, type Engine, type GuiSheets, type PackInfo } from '@engine'
import { createIndexedDbPackStore, createMemoryPackStore, type PackStore } from './packStore'
import {
  createPackManager,
  type GuiBytes,
  type PackDescription,
  type PackTarget,
} from './texturePack'

function guiBytes(sheets: GuiSheets): GuiBytes {
  const bytes = {
    widgets: sheets.widgets,
    icons: sheets.icons,
    inventory: sheets.inventory,
    container: sheets.container,
    panel: sheets.panel,
    panelSlice: sheets.panelSlice,
  }
  sheets.free()
  return bytes
}

const pngUrl = (bytes: Uint8Array) =>
  URL.createObjectURL(new Blob([bytes as BlobPart], { type: 'image/png' }))

function describe(info: PackInfo): PackDescription {
  const description = {
    description: info.description,
    icon: info.icon,
    tileSize: info.tileSize,
    gui: guiBytes(info.gui),
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
  iconUrl: pngUrl,
  revokeIconUrl: (url) => URL.revokeObjectURL(url),
  gui: {
    standard: () => guiBytes(builtinGui()),
    url: pngUrl,
    revoke: (url) => URL.revokeObjectURL(url),
  },
})

export const useTexturePack = () => texturePacks
