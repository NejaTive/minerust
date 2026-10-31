import type { WorldStats } from '@engine'
import { shallowReactive } from 'vue'
import type { PlayerInfo, TimeInfo } from './types'

const NOTICE_MS = 2500

const initialState = () => ({
  fps: 0,
  stats: undefined as WorldStats | undefined,
  time: undefined as TimeInfo | undefined,
  player: undefined as PlayerInfo | undefined,
  ready: false,
  noWebGpu: undefined as string | undefined,
  notice: undefined as string | undefined,
  newerSave: false,
  locked: false,
  inventoryOpen: false,
  lockLost: false,
  busy: true,
  error: undefined as string | undefined,
})

export const gameState = shallowReactive(initialState())

export function resetGameState() {
  const { noWebGpu } = gameState
  Object.assign(gameState, initialState(), { noWebGpu })
}

let noticeTimer = 0

export function notify(text: string) {
  gameState.notice = text
  clearTimeout(noticeTimer)
  noticeTimer = window.setTimeout(() => (gameState.notice = undefined), NOTICE_MS)
}

export function useGameState() {
  return gameState
}
