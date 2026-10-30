import { describe, expect, it } from 'vitest'
import { inventoryFlags, type InventoryMode } from './inventory-mode'

describe('inventoryFlags', () => {
  it('экран открыт и в inventory, и пока идёт возврат курсора', () => {
    expect(inventoryFlags('inventory').inventoryOpen).toBe(true)
    expect(inventoryFlags('closing').inventoryOpen).toBe(true)
  })

  it('в остальных режимах экрана нет', () => {
    const modes: InventoryMode[] = ['idle', 'grace', 'lock-lost']
    for (const mode of modes) expect(inventoryFlags(mode).inventoryOpen).toBe(false)
  })

  it('lockLost — только в своём режиме', () => {
    expect(inventoryFlags('lock-lost').lockLost).toBe(true)
    const others: InventoryMode[] = ['idle', 'inventory', 'closing', 'grace']
    for (const mode of others) expect(inventoryFlags(mode).lockLost).toBe(false)
  })
})
