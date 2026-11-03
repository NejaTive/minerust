export type ConsoleKey = 'submit' | 'complete' | 'history-up' | 'history-down' | 'close' | 'none'

export function classifyConsoleKey(code: string): ConsoleKey {
  switch (code) {
    case 'Enter':
    case 'NumpadEnter':
      return 'submit'
    case 'Tab':
      return 'complete'
    case 'ArrowUp':
      return 'history-up'
    case 'ArrowDown':
      return 'history-down'
    case 'Escape':
      return 'close'
    default:
      return 'none'
  }
}
