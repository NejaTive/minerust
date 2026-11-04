import 'fake-indexeddb/auto'
import { describe, expect, it, vi } from 'vitest'
import { createIndexedDbPackStore, createMemoryPackStore } from './packStore'
import {
  createPackManager,
  type GuiBytes,
  type PackDescription,
  type PackTarget,
} from './texturePack'

const bytes = (...values: number[]) => new Uint8Array(values)

function setup(parse?: (b: Uint8Array) => PackDescription) {
  const store = createMemoryPackStore()
  const manager = createPackManager({
    store,
    parse: parse ?? (() => ({ description: 'Описание', tileSize: 32 })),
  })
  const target: PackTarget = {
    loadTexturePack: vi.fn(() => ({ description: 'Описание', tileSize: 32 })),
    resetTexturePack: vi.fn(),
  }
  return { store, manager, target }
}

describe('менеджер текстур-пака', () => {
  it('устанавливает пак: сохраняет, показывает и применяет к движку', async () => {
    const { store, manager, target } = setup()
    await manager.attach(target)
    expect(await manager.install('pack.zip', bytes(1, 2))).toBe(true)
    expect(manager.state).toMatchObject({ active: true, name: 'pack.zip', tileSize: 32 })
    expect(target.loadTexturePack).toHaveBeenCalledWith(bytes(1, 2))
    expect(await store.load()).toEqual({ name: 'pack.zip', bytes: bytes(1, 2) })
  })

  it('при ошибке разбора ничего не меняет и показывает текст', async () => {
    const { store, manager, target } = setup(() => {
      throw new Error('нет текстур')
    })
    await manager.attach(target)
    expect(await manager.install('bad.zip', bytes(9))).toBe(false)
    expect(manager.state).toMatchObject({ active: false, error: 'нет текстур', busy: false })
    expect(target.loadTexturePack).not.toHaveBeenCalled()
    expect(await store.load()).toBeNull()
  })

  it('применяет сохранённый пак при подключении движка', async () => {
    const { store, manager, target } = setup()
    await store.save({ name: 'saved.zip', bytes: bytes(7) })
    await manager.attach(target)
    expect(manager.state).toMatchObject({ active: true, name: 'saved.zip' })
    expect(target.loadTexturePack).toHaveBeenCalledWith(bytes(7))
  })

  it('сброс очищает хранилище и возвращает стандартные текстуры', async () => {
    const { store, manager, target } = setup()
    await manager.attach(target)
    await manager.install('pack.zip', bytes(1))
    await manager.reset()
    expect(manager.state).toMatchObject({ active: false, name: '', tileSize: 32 })
    expect(target.resetTexturePack).toHaveBeenCalled()
    expect(await store.load()).toBeNull()
  })

  it('без движка пак запоминается и применяется позже', async () => {
    const { manager, target } = setup()
    await manager.install('pack.zip', bytes(3))
    expect(target.loadTexturePack).not.toHaveBeenCalled()
    await manager.attach(target)
    expect(target.loadTexturePack).toHaveBeenCalledWith(bytes(3))
  })

  it('сохранённый пак виден без движка, сброс очищает хранилище', async () => {
    const { store, manager } = setup()
    await store.save({ name: 'saved.zip', bytes: bytes(5) })
    await manager.restore()
    expect(manager.state).toMatchObject({
      active: true,
      name: 'saved.zip',
      description: 'Описание',
      tileSize: 32,
    })
    await manager.reset()
    expect(manager.state).toMatchObject({ active: false, name: '', tileSize: 32 })
    expect(await store.load()).toBeNull()
  })

  it('увеличивает revision при каждом применении', async () => {
    const { manager, target } = setup()
    await manager.attach(target)
    const before = manager.state.revision
    await manager.install('a.zip', bytes(1))
    expect(manager.state.revision).toBeGreaterThan(before)
  })
})

describe('хранилище пака в IndexedDB', () => {
  it('сохраняет, читает и очищает', async () => {
    const store = createIndexedDbPackStore()
    expect(await store.load()).toBeNull()
    await store.save({ name: 'a.zip', bytes: bytes(1, 2, 3) })
    expect(await store.load()).toEqual({ name: 'a.zip', bytes: bytes(1, 2, 3) })
    await store.clear()
    expect(await store.load()).toBeNull()
  })
})

describe('интерфейс из пака', () => {
  const gui = (tag: number): GuiBytes => ({
    widgets: bytes(tag, 1),
    icons: bytes(tag, 2),
    inventory: bytes(tag, 3),
    container: bytes(tag, 5),
    panel: bytes(tag, 4),
    panelSlice: 8,
  })

  function guiSetup() {
    const urls: string[] = []
    const revoked: string[] = []
    const manager = createPackManager({
      store: createMemoryPackStore(),
      parse: () => ({ description: '', tileSize: 32, gui: gui(7) }),
      gui: {
        standard: () => gui(0),
        url: (b) => {
          const url = `blob:${b[0]}-${b[1]}-${urls.length}`
          urls.push(url)
          return url
        },
        revoke: (url) => revoked.push(url),
      },
    })
    return { manager, urls, revoked }
  }

  it('не обращается к wasm при создании, встроенный интерфейс — при restore', async () => {
    const { manager, urls } = guiSetup()
    expect(urls).toHaveLength(0)
    expect(manager.state.gui).toBeUndefined()
    await manager.restore()
    expect(urls).toHaveLength(5)
    expect(manager.state.gui).toMatchObject({ widgets: 'blob:0-1-0', panelSlice: 8 })
  })

  it('пак подменяет интерфейс и освобождает старые URL, сброс возвращает встроенный', async () => {
    const { manager, urls, revoked } = guiSetup()
    await manager.restore()
    await manager.install('pack.zip', bytes(1))
    expect(manager.state.gui?.widgets).toBe('blob:7-1-5')
    expect(revoked).toEqual(urls.slice(0, 5))
    await manager.reset()
    expect(manager.state.gui?.widgets).toMatch(/^blob:0-1-/)
    expect(revoked).toHaveLength(10)
  })

  it('сохранённый пак виден без движка', async () => {
    const { manager } = guiSetup()
    await manager.install('pack.zip', bytes(1))
    expect(manager.state.gui?.icons).toBe('blob:7-2-6')
  })
})
