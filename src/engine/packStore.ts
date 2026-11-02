const DB_NAME = 'minerust-textures'
const STORE = 'pack'
const KEY = 'current'

export interface StoredPack {
  name: string
  bytes: Uint8Array
}

export interface PackStore {
  load(): Promise<StoredPack | null>
  save(pack: StoredPack): Promise<void>
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

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 1)
    req.onupgradeneeded = () => req.result.createObjectStore(STORE)
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
}

export function createIndexedDbPackStore(): PackStore {
  let db: Promise<IDBDatabase> | undefined
  const open = () => (db ??= openDb())
  return {
    async load() {
      const tx = (await open()).transaction(STORE, 'readonly')
      return (await request<StoredPack | undefined>(tx.objectStore(STORE).get(KEY))) ?? null
    },
    async save(pack) {
      const tx = (await open()).transaction(STORE, 'readwrite')
      tx.objectStore(STORE).put(pack, KEY)
      await completion(tx)
    },
    async clear() {
      const tx = (await open()).transaction(STORE, 'readwrite')
      tx.objectStore(STORE).delete(KEY)
      await completion(tx)
    },
  }
}

export function createMemoryPackStore(): PackStore {
  let stored: StoredPack | null = null
  return {
    load: async () => stored,
    save: async (pack) => void (stored = pack),
    clear: async () => void (stored = null),
  }
}
