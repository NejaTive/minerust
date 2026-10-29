export async function requestPointerLock(el: HTMLElement | undefined): Promise<boolean> {
  if (!el) return false
  try {
    await (el.requestPointerLock() as Promise<void> | undefined)
  } catch {
    return false
  }
  return true
}

export function releasePointerLock() {
  if (document.pointerLockElement) document.exitPointerLock()
}
