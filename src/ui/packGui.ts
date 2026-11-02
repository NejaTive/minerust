import { watchEffect } from 'vue'
import type { PackState } from '../engine/texturePack'

interface StyleTarget {
  style: { setProperty(name: string, value: string): void }
}

export function bindPackGui(state: Pick<PackState, 'gui'>, root: StyleTarget) {
  return watchEffect(
    () => {
      const gui = state.gui
      if (!gui) return
      const set = (name: string, value: string) => root.style.setProperty(name, value)
      set('--gui-widgets', `url("${gui.widgets}")`)
      set('--gui-icons', `url("${gui.icons}")`)
      set('--gui-inventory', `url("${gui.inventory}")`)
      set('--gui-panel', `url("${gui.panel}")`)
      set('--gui-panel-slice', String(gui.panelSlice))
    },
    { flush: 'sync' },
  )
}
