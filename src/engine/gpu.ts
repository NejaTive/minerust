export class NoWebGpuError extends Error {}

export function hasWebGpu(): boolean {
  if (import.meta.env.DEV && new URLSearchParams(location.search).has('nogpu')) return false
  return 'gpu' in navigator
}
