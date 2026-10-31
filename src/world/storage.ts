import type { GameMode, WorldType } from './kinds'

const DB_NAME = 'minerust'
const DB_VERSION = 2
const WORLDS = 'worlds'
const COLUMNS = 'world-columns'
const LEGACY_STORES = ['meta', 'columns']

export interface WorldMeta {
  version: number
  seed: number
  name: string
  mode: GameMode
  worldType: WorldType
  createdAt: number
  lastPlayed: number
  player?: number[]
  time?: number
  weather?: number[]
}

export interface StoredWorld extends WorldMeta {
  id: string
}

export interface NewWorld {
  name: string
  seed: number
  mode: GameMode
  worldType: WorldType
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
  load(): Promise<SavedWorld | null>
  save(meta: WorldMeta, columns: SavedColumn[]): Promise<void>
  clear(): Promise<void>
}

export interface WorldsStore {
  list(): Promise<StoredWorld[]>
  create(init: NewWorld, formatVersion: number): Promise<StoredWorld>
  remove(id: string): Promise<void>
  open(id: string): WorldStorage
}

const byRecentlyPlayed = (a: StoredWorld, b: StoredWorld) => b.lastPlayed - a.lastPlayed

function newRecord(init: NewWorld, formatVersion: number): StoredWorld {
  const now = Date.now()
  return {
    ...init,
    id: crypto.randomUUID(),
    version: formatVersion,
    createdAt: now,
    lastPlayed: now,
  }
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

const columnRange = (id: string) => IDBKeyRange.bound([id], [id, []])

export async function openStorage(): Promise<WorldsStore | null> {
  let db: IDBDatabase
  try {
    db = await new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open(DB_NAME, DB_VERSION)
      req.onupgradeneeded = () => {
        const created = req.result
        for (const legacy of LEGACY_STORES) {
          if (created.objectStoreNames.contains(legacy)) created.deleteObjectStore(legacy)
        }
        if (!created.objectStoreNames.contains(WORLDS)) created.createObjectStore(WORLDS)
        if (!created.objectStoreNames.contains(COLUMNS)) created.createObjectStore(COLUMNS)
      }
      req.onsuccess = () => resolve(req.result)
      req.onerror = () => reject(req.error)
    })
  } catch {
    return null
  }

  return {
    async list() {
      const tx = db.transaction(WORLDS, 'readonly')
      const worlds = await request<StoredWorld[]>(tx.objectStore(WORLDS).getAll())
      return worlds.sort(byRecentlyPlayed)
    },
    async create(init, formatVersion) {
      const record = newRecord(init, formatVersion)
      const tx = db.transaction(WORLDS, 'readwrite')
      tx.objectStore(WORLDS).put(record, record.id)
      await completion(tx)
      return record
    },
    remove(id) {
      const tx = db.transaction([WORLDS, COLUMNS], 'readwrite')
      tx.objectStore(WORLDS).delete(id)
      tx.objectStore(COLUMNS).delete(columnRange(id))
      return completion(tx)
    },
    open(id) {
      return {
        async load() {
          const tx = db.transaction([WORLDS, COLUMNS], 'readonly')
          const [meta, columns] = await Promise.all([
            request<StoredWorld | undefined>(tx.objectStore(WORLDS).get(id)),
            request<SavedColumn[]>(tx.objectStore(COLUMNS).getAll(columnRange(id))),
          ])
          return meta ? { meta, columns } : null
        },
        save(meta, columns) {
          const tx = db.transaction([WORLDS, COLUMNS], 'readwrite')
          tx.objectStore(WORLDS).put({ ...meta, id }, id)
          const store = tx.objectStore(COLUMNS)
          for (const column of columns) store.put(column, [id, column.x, column.z])
          return completion(tx)
        },
        clear() {
          const tx = db.transaction(COLUMNS, 'readwrite')
          tx.objectStore(COLUMNS).delete(columnRange(id))
          return completion(tx)
        },
      }
    },
  }
}

export function createMemoryStorage(): WorldsStore {
  const worlds = new Map<string, StoredWorld>()
  const columns = new Map<string, Map<string, SavedColumn>>()
  const columnsOf = (id: string) => {
    let own = columns.get(id)
    if (!own) columns.set(id, (own = new Map()))
    return own
  }
  return {
    async list() {
      return [...worlds.values()].sort(byRecentlyPlayed)
    },
    async create(init, formatVersion) {
      const record = newRecord(init, formatVersion)
      worlds.set(record.id, record)
      return record
    },
    async remove(id) {
      worlds.delete(id)
      columns.delete(id)
    },
    open(id) {
      return {
        async load() {
          const meta = worlds.get(id)
          return meta ? { meta, columns: [...columnsOf(id).values()] } : null
        },
        async save(meta, saved) {
          worlds.set(id, { ...meta, id })
          for (const column of saved) columnsOf(id).set(`${column.x},${column.z}`, column)
        },
        async clear() {
          columns.delete(id)
        },
      }
    },
  }
}
