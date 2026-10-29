export interface TimeInfo {
  time: number
  phase: string
}

export interface PlayerInfo {
  mode: 'walking' | 'flying' | 'noclip'
  onGround: boolean
}
