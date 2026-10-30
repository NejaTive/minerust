export type InventoryMode = 'idle' | 'inventory' | 'closing' | 'grace' | 'lock-lost'

export function inventoryFlags(mode: InventoryMode): { inventoryOpen: boolean; lockLost: boolean } {
  return {
    inventoryOpen: mode === 'inventory' || mode === 'closing',
    lockLost: mode === 'lock-lost',
  }
}
