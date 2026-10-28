import { describe, expect, it, vi } from 'vitest'
import {
  DEFAULT_SEED,
  WorldPersistence,
  type PersistenceDeps,
  type SaveableEngine,
} from './persistence'
import type { SavedColumn, SavedWorld, WorldMeta, WorldStorage } from './storage'

const goodMeta: WorldMeta = { version: 2, seed: 7, player: [1, 2, 3, 0, 0, 0], time: 0.5 }

function fakeStorage(saved: SavedWorld | null = null) {
  const written: { meta: WorldMeta; columns: SavedColumn[] }[] = []
  const storage: WorldStorage = {
    load: async () => saved,
    save: async (meta, columns) => void written.push({ meta, columns }),
    clear: vi.fn(async () => {}),
  }
  return { storage, written }
}

function setup(storage: WorldStorage | null, formatVersion = 2) {
  const notes: string[] = []
  const newer: boolean[] = []
  const deps: PersistenceDeps = {
    open: async () => storage,
    formatVersion,
    notify: (text) => notes.push(text),
    setNewerSave: (value) => newer.push(value),
    randomSeed: () => 42,
  }
  return { persistence: new WorldPersistence(deps), notes, newer }
}

function fakeEngine(dirty: number[] = [], bytes = new Uint8Array([1])): SaveableEngine {
  return {
    drain_dirty: () => dirty,
    export_column: () => bytes,
    import_column: vi.fn(),
    seed: () => 7,
    camera_state: () => [1, 2, 3, 0, 0, 0],
    time_of_day: () => 0.25,
  }
}

describe('WorldPersistence.load', () => {
  it('без хранилища — мир по умолчанию и предупреждение', async () => {
    const { persistence, notes } = setup(null)
    expect(await persistence.load()).toEqual({ seed: DEFAULT_SEED })
    expect(notes).toHaveLength(1)
  })

  it('пустая база — сид по умолчанию', async () => {
    const { persistence } = setup(fakeStorage().storage)
    expect(await persistence.load()).toEqual({ seed: DEFAULT_SEED })
  })

  it('мир той же и более старой версии загружается', async () => {
    for (const version of [2, 1]) {
      const meta = { ...goodMeta, version }
      const { persistence } = setup(fakeStorage({ meta, columns: [] }).storage)
      expect(await persistence.load()).toBe(meta)
    }
  })

  it('мир новее не загружается и не перезаписывается', async () => {
    const meta = { ...goodMeta, version: 9 }
    const { storage, written } = fakeStorage({ meta, columns: [] })
    const { persistence, newer } = setup(storage)
    expect(await persistence.load()).toEqual({ seed: DEFAULT_SEED })
    expect(newer).toEqual([true])
    await persistence.save(fakeEngine())
    expect(written).toHaveLength(0)
    expect(storage.clear).not.toHaveBeenCalled()
  })

  it('«Новый мир» снимает защиту нового сохранения', async () => {
    const meta = { ...goodMeta, version: 9 }
    const { storage, written } = fakeStorage({ meta, columns: [] })
    const { persistence, newer } = setup(storage)
    await persistence.load()
    await persistence.reset()
    expect(storage.clear).toHaveBeenCalled()
    expect(newer).toEqual([true, false])
    await persistence.save(fakeEngine())
    expect(written).toHaveLength(1)
  })

  it('битые метаданные — база очищается, сид случайный', async () => {
    const meta = { ...goodMeta, seed: Number.NaN }
    const { storage } = fakeStorage({ meta, columns: [] })
    const { persistence, notes } = setup(storage)
    expect(await persistence.load()).toEqual({ seed: 42 })
    expect(storage.clear).toHaveBeenCalled()
    expect(notes).toHaveLength(1)
  })

  it('база не читается — сохранения выключаются', async () => {
    const storage: WorldStorage = {
      load: async () => {
        throw new Error('boom')
      },
      save: vi.fn(),
      clear: vi.fn(),
    }
    vi.spyOn(console, 'warn').mockImplementation(() => {})
    const { persistence } = setup(storage)
    expect(await persistence.load()).toEqual({ seed: DEFAULT_SEED })
    await persistence.save(fakeEngine())
    expect(storage.save).not.toHaveBeenCalled()
  })
})

describe('WorldPersistence.save / importInto', () => {
  it('записывает изменённые колонки и состояние игрока', async () => {
    const { storage, written } = fakeStorage()
    const { persistence } = setup(storage)
    await persistence.load()
    await persistence.save(fakeEngine([3, -4]))
    expect(written[0].meta).toEqual({ version: 2, seed: 7, player: [1, 2, 3, 0, 0, 0], time: 0.25 })
    expect(written[0].columns.map((c) => [c.x, c.z])).toEqual([[3, -4]])
  })

  it('колонки из базы импортируются в движок, битые пропускаются', async () => {
    const good: SavedColumn = { x: 1, z: 2, bytes: new Uint8Array([9]) }
    const bad = { x: 1.5, z: 0, bytes: new Uint8Array() } as SavedColumn
    const { storage } = fakeStorage({ meta: goodMeta, columns: [good, bad] })
    const { persistence, notes } = setup(storage)
    await persistence.load()
    vi.spyOn(console, 'warn').mockImplementation(() => {})
    const engine = fakeEngine()
    persistence.importInto(engine)
    expect(engine.import_column).toHaveBeenCalledTimes(1)
    expect(engine.import_column).toHaveBeenCalledWith(1, 2, good.bytes)
    expect(notes).toEqual(['Повреждённые колонки пропущены: 1'])
  })
})
