import init, { Engine, blocks, default_settings, settings_limits } from '@engine'
import type { EngineSettings, SettingsLimits } from '../settings/settings'

export interface BlockInfo {
  name: string
  label: string
  placeable: boolean
}

export interface EngineInfo {
  formatVersion: number
  blocks: BlockInfo[]
  limits: SettingsLimits
  defaults: EngineSettings
}

let loaded: EngineInfo | undefined

export async function loadEngineInfo(): Promise<EngineInfo> {
  await init()
  const list = blocks()
  const limits = settings_limits()
  const defaults = default_settings()
  loaded = {
    formatVersion: Engine.format_version(),
    blocks: list.map(({ name, label, placeable }) => ({ name, label, placeable })),
    limits: {
      renderDistanceMin: limits.renderDistanceMin,
      renderDistanceMax: limits.renderDistanceMax,
      mouseSensitivityMin: limits.mouseSensitivityMin,
      mouseSensitivityMax: limits.mouseSensitivityMax,
      fovMin: limits.fovMin,
      fovMax: limits.fovMax,
    },
    defaults: {
      renderDistance: defaults.renderDistance,
      ambientOcclusion: defaults.ambientOcclusion,
      clouds: defaults.clouds,
      weather: defaults.weather,
      autoStep: defaults.autoStep,
      mouseSensitivity: defaults.mouseSensitivity,
      fov: defaults.fov,
      coloredLight: defaults.coloredLight,
      waterReflections: defaults.waterReflections,
      groundFog: defaults.groundFog,
      plantSway: defaults.plantSway,
      hdr: defaults.hdr,
      bloom: defaults.bloom,
      waterDepth: defaults.waterDepth,
      sunRays: defaults.sunRays,
      shadows: defaults.shadows,
      shadowQualityHigh: defaults.shadowQualityHigh,
      cloudShadows: defaults.cloudShadows,
    },
  }
  list.forEach((block) => block.free())
  limits.free()
  defaults.free()
  return loaded
}

export function engineInfo(): EngineInfo {
  if (!loaded) throw new Error('loadEngineInfo() must finish before the app is mounted')
  return loaded
}

export function blockLabel(name: string): string {
  return engineInfo().blocks.find((block) => block.name === name)?.label ?? name
}
