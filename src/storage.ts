// Сохранение мира в IndexedDB: метаданные мира и байты изменённых колонок
// (`Engine.export_column`). Без зависимостей: IndexedDB обёрнут в промисы вручную.

const DB_NAME = 'minerust'
// Версия схемы базы (какие хранилища есть). Формат самих данных — `WorldMeta.version`;
// при его смене схема не меняется, сохранение просто очищается.
const DB_VERSION = 1
const META = 'meta'
const COLUMNS = 'columns'
const WORLD_KEY = 'world'

export interface WorldMeta {
  /** `Engine.format_version()` на момент записи. */
  version: number
  seed: number
  /** `Engine.camera_state()`: глаза x, y, z, yaw, pitch, режим. */
  player: number[]
}

export interface SavedColumn {
  x: number
  z: number
  bytes: Uint8Array
}

export interface SavedWorld {
  meta: WorldMeta
  columns: SavedColumn[]
}

export interface WorldStorage {
  /** `null` — сохранения нет. */
  load(): Promise<SavedWorld | null>
  /** Пишет метаданные и колонки одной транзакцией: при сбое не останется половины. */
  save(meta: WorldMeta, columns: SavedColumn[]): Promise<void>
  clear(): Promise<void>
}

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
}

function completion(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve()
    tx.onerror = () => reject(tx.error)
    tx.onabort = () => reject(tx.error ?? new Error('transaction aborted'))
  })
}

/** `null`, если IndexedDB недоступен: приватный режим, запрет в настройках и т. п. */
export async function openStorage(): Promise<WorldStorage | null> {
  let db: IDBDatabase
  try {
    db = await new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open(DB_NAME, DB_VERSION)
      req.onupgradeneeded = () => {
        const created = req.result
        if (!created.objectStoreNames.contains(META)) created.createObjectStore(META)
        if (!created.objectStoreNames.contains(COLUMNS)) created.createObjectStore(COLUMNS)
      }
      req.onsuccess = () => resolve(req.result)
      req.onerror = () => reject(req.error)
    })
  } catch {
    return null
  }

  return {
    async load() {
      const tx = db.transaction([META, COLUMNS], 'readonly')
      const [meta, columns] = await Promise.all([
        request<WorldMeta | undefined>(tx.objectStore(META).get(WORLD_KEY)),
        request<SavedColumn[]>(tx.objectStore(COLUMNS).getAll()),
      ])
      return meta ? { meta, columns } : null
    },
    // Транзакция создаётся синхронно, до первого await: так запись из pagehide успевает
    // начаться, пока страница жива. Пишущие транзакции с общими хранилищами IndexedDB
    // выполняет строго по очереди создания, поэтому сохранения не накладываются.
    save(meta, columns) {
      const tx = db.transaction([META, COLUMNS], 'readwrite')
      tx.objectStore(META).put(meta, WORLD_KEY)
      const store = tx.objectStore(COLUMNS)
      for (const column of columns) store.put(column, `${column.x},${column.z}`)
      return completion(tx)
    },
    clear() {
      const tx = db.transaction([META, COLUMNS], 'readwrite')
      tx.objectStore(META).clear()
      tx.objectStore(COLUMNS).clear()
      return completion(tx)
    },
  }
}
