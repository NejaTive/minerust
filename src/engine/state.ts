import type { WorldStats } from '@engine'
import { shallowReactive } from 'vue'
import type { PlayerInfo, TimeInfo } from './types'

const NOTICE_MS = 2500

/** Показания движка для интерфейса. Один движок на приложение, поэтому состояние общее. */
export const gameState = shallowReactive({
  fps: 0,
  stats: undefined as WorldStats | undefined,
  time: undefined as TimeInfo | undefined,
  player: undefined as PlayerInfo | undefined,
  /** Движок создан и рисует: можно показывать хотбар. */
  ready: false,
  /** WebGPU нет; пустая строка — причина неизвестна (нет `navigator.gpu`). */
  noWebGpu: undefined as string | undefined,
  notice: undefined as string | undefined,
  /** В базе мир более новой версии игры: он не загружен и не перезаписывается. */
  newerSave: false,
  /** Курсор захвачен (игра идёт); иначе пауза и меню. */
  locked: false,
  /** Движок грузится или пересоздаётся, мир менять нельзя. */
  busy: true,
  error: undefined as string | undefined,
})

let noticeTimer = 0

export function notify(text: string) {
  gameState.notice = text
  clearTimeout(noticeTimer)
  noticeTimer = window.setTimeout(() => (gameState.notice = undefined), NOTICE_MS)
}

export function useGameState() {
  return gameState
}
