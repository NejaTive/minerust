export interface TimeInfo {
  time: number
  phase: string
  weather: string
}

export interface PlayerInfo {
  mode: 'walking' | 'flying' | 'noclip'
  onGround: boolean
}
