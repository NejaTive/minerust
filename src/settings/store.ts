/** Хранилище строк под ключами; в тестах подменяется памятью. */
export interface KeyValueStore {
  get(key: string): string | null
  set(key: string, value: string): void
  remove(key: string): void
}

// localStorage может бросать (приватный режим, запрет site data) — тогда работаем
// со значениями по умолчанию и ничего не запоминаем.
export const localStore: KeyValueStore = {
  get(key) {
    try {
      return localStorage.getItem(key)
    } catch {
      return null
    }
  },
  set(key, value) {
    try {
      localStorage.setItem(key, value)
    } catch {
      // Значение просто не переживёт перезагрузку.
    }
  },
  remove(key) {
    try {
      localStorage.removeItem(key)
    } catch {
      // Нечего чистить.
    }
  },
}

/** Хранилище в памяти: для тестов и как запасной вариант. */
export function memoryStore(initial: Record<string, string> = {}): KeyValueStore {
  const data = new Map(Object.entries(initial))
  return {
    get: (key) => data.get(key) ?? null,
    set: (key, value) => void data.set(key, value),
    remove: (key) => void data.delete(key),
  }
}
