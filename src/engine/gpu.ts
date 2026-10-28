/** Движок сообщил, что WebGPU заявлен браузером, но адаптера или устройства нет. */
export class NoWebGpuError extends Error {}

/**
 * Есть ли `navigator.gpu`. `in`, а не `navigator.gpu`: `lib.dom` может не знать это поле.
 * `?nogpu` в dev-сборке имитирует браузер без WebGPU (в production не действует).
 */
export function hasWebGpu(): boolean {
  if (import.meta.env.DEV && new URLSearchParams(location.search).has('nogpu')) return false
  return 'gpu' in navigator
}
