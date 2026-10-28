export interface TimeInfo {
  /** Время суток 0..1. */
  time: number
  /** `day`, `dusk`, `night` или `dawn`. */
  phase: string
}

export interface PlayerInfo {
  mode: 'walking' | 'flying' | 'noclip'
  onGround: boolean
}
