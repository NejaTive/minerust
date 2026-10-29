const AUTOSAVE_MS = 5000

/**
 * Автосохранение раз в `AUTOSAVE_MS` и при уходе со страницы. Возвращает функцию отписки.
 * Запись при `pagehide` идёт «как получится»: ждать её браузер не станет.
 */
export function startAutosave(save: () => void): () => void {
  const id = window.setInterval(save, AUTOSAVE_MS)
  const onVisibilityChange = () => {
    if (document.visibilityState === 'hidden') save()
  }
  document.addEventListener('visibilitychange', onVisibilityChange)
  window.addEventListener('pagehide', save)
  return () => {
    clearInterval(id)
    document.removeEventListener('visibilitychange', onVisibilityChange)
    window.removeEventListener('pagehide', save)
  }
}
