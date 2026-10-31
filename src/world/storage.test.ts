import 'fake-indexeddb/auto'
import { beforeEach, describe, expect, it } from 'vitest'
import {
  createMemoryStorage,
  openStorage,
  type NewWorld,
  type SavedColumn,
  type WorldsStore,
} from './storage'

const init = (name: string, seed = 1): NewWorld => ({
  name,
  seed,
  mode: 'creative',
  worldType: 'standard',
})

const column = (x: number, z: number, byte: number): SavedColumn => ({
  x,
  z,
  bytes: new Uint8Array([byte]),
})

let idb: WorldsStore | undefined

describe.each([
  ['IndexedDB', async () => (idb ??= (await openStorage())!)],
  ['память', async () => createMemoryStorage()],
])('хранилище миров: %s', (_name, open) => {
  let store: WorldsStore

  beforeEach(async () => {
    store = await open()
    for (const world of await store.list()) await store.remove(world.id)
  })

  it('создаёт мир с метаданными и показывает его в списке', async () => {
    const created = await store.create(init('Мир', 77), 3)
    expect(created).toMatchObject({
      name: 'Мир',
      seed: 77,
      mode: 'creative',
      worldType: 'standard',
      version: 3,
    })
    expect(created.createdAt).toBe(created.lastPlayed)
    expect(await store.list()).toEqual([created])
  })

  it('список — по убыванию последней игры', async () => {
    const old = await store.create(init('Старый'), 1)
    const fresh = await store.create(init('Новый'), 1)
    await store.open(old.id).save({ ...old, lastPlayed: fresh.lastPlayed + 1000 }, [])
    expect((await store.list()).map((w) => w.name)).toEqual(['Старый', 'Новый'])
  })

  it('сохраняет и загружает мир вместе с колонками', async () => {
    const world = await store.create(init('Мир'), 1)
    const storage = store.open(world.id)
    await storage.save({ ...world, player: [1, 2, 3, 0, 0, 0] }, [
      column(0, 0, 1),
      column(-1, 2, 2),
    ])
    const loaded = await storage.load()
    expect(loaded?.meta.player).toEqual([1, 2, 3, 0, 0, 0])
    expect(loaded?.columns.map((c) => [c.x, c.z, c.bytes[0]]).sort()).toEqual([
      [-1, 2, 2],
      [0, 0, 1],
    ])
  })

  it('колонки миров изолированы', async () => {
    const a = await store.create(init('A'), 1)
    const b = await store.create(init('B'), 1)
    await store.open(a.id).save(a, [column(0, 0, 10)])
    await store.open(b.id).save(b, [column(0, 0, 20), column(1, 1, 21)])
    expect((await store.open(a.id).load())?.columns.map((c) => c.bytes[0])).toEqual([10])
    expect((await store.open(b.id).load())?.columns).toHaveLength(2)
  })

  it('удаление мира убирает только его', async () => {
    const a = await store.create(init('A'), 1)
    const b = await store.create(init('B'), 1)
    await store.open(a.id).save(a, [column(0, 0, 1)])
    await store.open(b.id).save(b, [column(0, 0, 2)])
    await store.remove(a.id)
    expect((await store.list()).map((w) => w.id)).toEqual([b.id])
    expect(await store.open(a.id).load()).toBeNull()
    expect((await store.open(b.id).load())?.columns).toHaveLength(1)
  })

  it('clear стирает колонки, метаданные остаются', async () => {
    const world = await store.create(init('A'), 1)
    const storage = store.open(world.id)
    await storage.save(world, [column(0, 0, 1)])
    await storage.clear()
    const loaded = await storage.load()
    expect(loaded?.meta.name).toBe('A')
    expect(loaded?.columns).toEqual([])
  })

  it('неизвестный мир не загружается', async () => {
    expect(await store.open('нет-такого').load()).toBeNull()
  })
})
