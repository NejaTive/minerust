const AUTOSAVE_MS = 5000

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
