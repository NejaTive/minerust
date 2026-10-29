/**
 * Режим креатив-инвентаря относительно захвата курсора. Один источник правды вместо набора
 * флагов: `gameState.inventoryOpen` и `lockLost` только выводятся из него (`inventoryFlags`).
 *
 * - `idle` — обычная игра или меню; паузу определяет сам захват курсора.
 * - `inventory` — открыт: курсор свободен, симуляция идёт, мир ввода не получает.
 * - `closing` — идёт запрос захвата курсора; экран ещё открыт, чтобы меню паузы не мелькнуло
 *   в промежутке между снятием экрана и `pointerlockchange`.
 * - `grace` — курсор вернули, но Chrome может снять его тем же нажатием Esc/E; потеря захвата
 *   в этом режиме — «нужен клик», а не меню (см. `INVENTORY_LOCK_GRACE_MS`).
 * - `lock-lost` — курсор вернуть не удалось: без меню и без паузы ждём клик по канвасу.
 */
export type InventoryMode = 'idle' | 'inventory' | 'closing' | 'grace' | 'lock-lost'

export function inventoryFlags(mode: InventoryMode): { inventoryOpen: boolean; lockLost: boolean } {
  return {
    inventoryOpen: mode === 'inventory' || mode === 'closing',
    lockLost: mode === 'lock-lost',
  }
}
