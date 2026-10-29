/**
 * Захватывает курсор; вызывать из обработчика действия пользователя (клик по кнопке меню).
 * Возвращает, удалось ли: Chrome около секунды после выхода по Esc отказывает.
 */
export async function requestPointerLock(el: HTMLElement | undefined): Promise<boolean> {
  if (!el) return false
  try {
    // Старые браузеры возвращают undefined, новые — Promise, который может реджектиться.
    await (el.requestPointerLock() as Promise<void> | undefined)
  } catch {
    return false
  }
  return true
}

export function releasePointerLock() {
  if (document.pointerLockElement) document.exitPointerLock()
}
