import { checkMeta } from './meta'
import { type SavedColumn, type WorldMeta, type WorldStorage } from './storage'

/** Сид мира, пока нет сохранения: первый запуск даёт привычный рельеф. */
export const DEFAULT_SEED = 1337

/** Часть `Engine`, нужная сохранению; в тестах подменяется заглушкой. */
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
  /** `Engine.format_version()`. */
  formatVersion: number
  notify(text: string): void
  /** В базе мир более новой версии игры: он не загружен и не перезаписывается. */
  setNewerSave(newer: boolean): void
  randomSeed(): number
}

export interface WorldStart {
  seed: number
  player?: number[]
  time?: number
}

const columnKey = (column: { x: number; z: number }) => `${column.x},${column.z}`

/** Сохранение мира в IndexedDB: колонки, игрок, время; загрузка и сброс для «Нового мира». */
export class WorldPersistence {
  // `null` — IndexedDB недоступен, игра идёт без сохранений.
  private storage: WorldStorage | null = null
  // В базе мир, записанный более новой версией игры: не читаем и не пишем поверх, пока
  // игрок явно не начнёт новый мир (`reset` сотрёт базу после подтверждения).
  private newerSave = false
  // Последние байты каждой изменённой колонки текущего мира (и прочитанных из базы).
  private readonly worldColumns = new Map<string, SavedColumn>()
  // Колонки, снятые с движка, но ещё не записанные. При ошибке записи остаются здесь
  // и уходят со следующим сохранением: `drain_dirty` второй раз их не отдаст.
  private readonly unsaved = new Map<string, SavedColumn>()
  // Последнее начатое сохранение; IndexedDB выполняет их по очереди, так что его
  // завершение означает, что записаны и все предыдущие.
  private saving: Promise<void> = Promise.resolve()
  private saveFailed = false

  constructor(private readonly deps: PersistenceDeps) {}

  /** Открывает хранилище и читает сохранённый мир в память. */
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
      // Не прочитали — не пишем поверх: в базе мог остаться чужой мир.
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

  /** Правки — в только что созданный движок, до первого кадра. */
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

  /**
   * Сохраняет несохранённые колонки и игрока (позицию — каждый раз, даже без правок).
   * Состояние снимается синхронно, запись идёт в фоне; ошибки не пробрасываются.
   */
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
        // Сравнение по ссылке: пока шла запись, колонку могли снять с движка заново.
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

  /**
   * Подготовка к новому миру; движок к этому моменту уже остановлен, чтобы автосохранение
   * не записало старый мир поверх пустой базы. Начатая запись должна закончиться до `clear`.
   */
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

  /** Снимает с движка несохранённые колонки в `worldColumns` и `unsaved`. */
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

  /**
   * Очищает базу. Если не вышло, сохранения выключаются: иначе старые колонки
   * подмешались бы к новому миру при следующей загрузке.
   */
  private async clearStorage() {
    try {
      await this.storage?.clear()
    } catch (e) {
      console.warn('world clear failed', e)
      this.storage = null
    }
  }
}
