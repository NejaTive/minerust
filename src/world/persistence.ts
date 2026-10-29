import { checkMeta } from './meta'
import { type SavedColumn, type WorldMeta, type WorldStorage } from './storage'

export const DEFAULT_SEED = 1337

export interface SaveableEngine {
  drain_dirty(): ArrayLike<number>
  export_column(x: number, z: number): Uint8Array | undefined
  import_column(x: number, z: number, bytes: Uint8Array): void
  seed(): number
  camera_state(): ArrayLike<number>
  time_of_day(): number
}

export interface PersistenceDeps {
  open(): Promise<WorldStorage | null>
  formatVersion: number
  notify(text: string): void
  setNewerSave(newer: boolean): void
  randomSeed(): number
}

export interface WorldStart {
  seed: number
  player?: number[]
  time?: number
}

const columnKey = (column: { x: number; z: number }) => `${column.x},${column.z}`

export class WorldPersistence {
  private storage: WorldStorage | null = null
  private newerSave = false
  private readonly worldColumns = new Map<string, SavedColumn>()
  private readonly unsaved = new Map<string, SavedColumn>()
  private saving: Promise<void> = Promise.resolve()
  private saveFailed = false

  constructor(private readonly deps: PersistenceDeps) {}

  async load(): Promise<WorldStart> {
    this.storage = await this.deps.open()
    if (!this.storage) {
      this.deps.notify('Сохранения недоступны — мир не сохранится')
      return { seed: DEFAULT_SEED }
    }
    let saved
    try {
      saved = await this.storage.load()
    } catch (e) {
      console.warn('world load failed', e)
      this.storage = null
      this.deps.notify('Сохранение не читается — мир не сохранится')
      return { seed: DEFAULT_SEED }
    }
    if (!saved) return { seed: DEFAULT_SEED }
    const status = checkMeta(saved.meta, this.deps.formatVersion)
    if (status === 'broken') {
      await this.clearStorage()
      this.deps.notify('Сохранение повреждено — начат новый мир')
      return { seed: this.deps.randomSeed() }
    }
    if (status === 'newer') {
      this.newerSave = true
      this.deps.setNewerSave(true)
      this.deps.notify('Сохранение из более новой версии игры — мир не загружен')
      return { seed: DEFAULT_SEED }
    }
    for (const column of saved.columns) this.worldColumns.set(columnKey(column), column)
    return saved.meta
  }

  importInto(engine: SaveableEngine) {
    let broken = 0
    for (const [key, column] of this.worldColumns) {
      try {
        if (!Number.isInteger(column.x) || !Number.isInteger(column.z)) throw new Error('bad key')
        if (!(column.bytes instanceof Uint8Array)) throw new Error('bad bytes')
        engine.import_column(column.x, column.z, column.bytes)
      } catch (e) {
        console.warn(`saved column ${key} skipped`, e)
        this.worldColumns.delete(key)
        broken++
      }
    }
    if (broken > 0) this.deps.notify(`Повреждённые колонки пропущены: ${broken}`)
  }

  save(engine: SaveableEngine | undefined): Promise<void> {
    const store = this.storage
    if (!engine || !store || this.newerSave) return this.saving
    this.collectColumns(engine)
    const meta: WorldMeta = {
      version: this.deps.formatVersion,
      seed: engine.seed(),
      player: Array.from(engine.camera_state()),
      time: engine.time_of_day(),
    }
    const columns = [...this.unsaved.values()]
    this.saving = store.save(meta, columns).then(
      () => {
        this.saveFailed = false
        for (const column of columns) {
          if (this.unsaved.get(columnKey(column)) === column) this.unsaved.delete(columnKey(column))
        }
      },
      (e: unknown) => {
        console.warn('world save failed', e)
        if (!this.saveFailed) this.deps.notify('Не удалось сохранить мир')
        this.saveFailed = true
      },
    )
    return this.saving
  }

  async reset() {
    await this.saving
    this.worldColumns.clear()
    this.unsaved.clear()
    await this.clearStorage()
    if (this.storage) {
      this.newerSave = false
      this.deps.setNewerSave(false)
    }
  }

  private collectColumns(engine: SaveableEngine) {
    const keys = engine.drain_dirty()
    for (let i = 0; i < keys.length; i += 2) {
      const x = keys[i]
      const z = keys[i + 1]
      const bytes = engine.export_column(x, z)
      if (!bytes) continue
      const column = { x, z, bytes }
      this.worldColumns.set(columnKey(column), column)
      this.unsaved.set(columnKey(column), column)
    }
  }

  private async clearStorage() {
    try {
      await this.storage?.clear()
    } catch (e) {
      console.warn('world clear failed', e)
      this.storage = null
    }
  }
}
