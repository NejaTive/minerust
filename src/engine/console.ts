import type { Engine } from '@engine'
import { ref, shallowReactive } from 'vue'
import { CommandHistory } from '../console/history'
import { appendLog, type LogEntry } from '../console/log'
import type { Completion } from '../console/complete'

export const consoleLog = shallowReactive({ entries: [] as LogEntry[] })
export const consoleInput = ref('/')
export const commandHistory = new CommandHistory()

let host: Engine | undefined
let closeScreen: (() => void) | undefined
let nextId = 0

export function bindConsole(engine: Engine | undefined, close?: () => void) {
  host = engine
  closeScreen = close
}

export function closeConsole() {
  closeScreen?.()
}

export function resetConsoleInput() {
  consoleInput.value = '/'
}

export function resetConsole() {
  resetConsoleInput()
  consoleLog.entries = []
  commandHistory.reset()
  commandHistory.clear()
}

function log(text: string, ok: boolean, suggest?: string) {
  consoleLog.entries = appendLog(consoleLog.entries, text, ok, performance.now(), nextId, suggest)
  nextId += text.split('\n').length
}

export function submitCommand(line: string) {
  commandHistory.push(line)
  if (!host) return
  const result = host.run_command(line)
  log(result.text, result.ok, result.suggest ?? undefined)
  result.free()
}

export function completeCommand(line: string): Completion {
  if (!host) return { start: 0, options: [] }
  const result = host.complete_command(line)
  const completion = { start: result.start, options: result.options }
  result.free()
  return completion
}

export function commandUsage(line: string): string {
  return host?.command_usage(line) ?? ''
}
