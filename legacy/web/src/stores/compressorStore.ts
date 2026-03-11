// Dynamics compressor state — threshold, knee, ratio, attack, release
import { create } from 'zustand'
import { loadJson } from '../utils/storage'

interface CompressorPreset {
  threshold: number
  knee: number
  ratio: number
  attack: number
  release: number
}

const BUILT_IN_PRESETS: Record<string, CompressorPreset> = {
  Speech:       { threshold: -20, knee: 20, ratio: 8, attack: 0.005, release: 0.15 },
  'Night Mode': { threshold: -30, knee: 10, ratio: 20, attack: 0.001, release: 0.1 },
  Gentle:       { threshold: -18, knee: 30, ratio: 4, attack: 0.01, release: 0.3 },
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value))
}

interface CompressorState {
  enabled: boolean
  activePreset: string | null
  threshold: number
  knee: number
  ratio: number
  attack: number
  release: number

  setEnabled: (enabled: boolean) => void
  setPreset: (name: string) => void
  setParam: (key: 'threshold' | 'knee' | 'ratio' | 'attack' | 'release', value: number) => void
  reset: () => void
}

const PARAM_RANGES = {
  threshold: [-100, 0],
  knee: [0, 40],
  ratio: [1, 20],
  attack: [0, 1],
  release: [0, 1],
} as const

export const useCompressorStore = create<CompressorState>((set) => ({
  enabled: loadJson<boolean>('akroasis_compressor_enabled', false),
  activePreset: null,
  threshold: loadJson<number>('akroasis_compressor_threshold', -24),
  knee: loadJson<number>('akroasis_compressor_knee', 30),
  ratio: loadJson<number>('akroasis_compressor_ratio', 12),
  attack: loadJson<number>('akroasis_compressor_attack', 0.003),
  release: loadJson<number>('akroasis_compressor_release', 0.25),

  setEnabled: (enabled) => {
    localStorage.setItem('akroasis_compressor_enabled', JSON.stringify(enabled))
    set({ enabled })
  },

  setPreset: (name) => {
    const preset = BUILT_IN_PRESETS[name]
    if (!preset) return
    localStorage.setItem('akroasis_compressor_threshold', JSON.stringify(preset.threshold))
    localStorage.setItem('akroasis_compressor_knee', JSON.stringify(preset.knee))
    localStorage.setItem('akroasis_compressor_ratio', JSON.stringify(preset.ratio))
    localStorage.setItem('akroasis_compressor_attack', JSON.stringify(preset.attack))
    localStorage.setItem('akroasis_compressor_release', JSON.stringify(preset.release))
    set({ ...preset, activePreset: name, enabled: true })
    localStorage.setItem('akroasis_compressor_enabled', 'true')
  },

  setParam: (key, value) => {
    const [min, max] = PARAM_RANGES[key]
    const clamped = clamp(value, min, max)
    localStorage.setItem(`akroasis_compressor_${key}`, JSON.stringify(clamped))
    set({ [key]: clamped, activePreset: null })
  },

  reset: () => {
    const defaults = { threshold: -24, knee: 30, ratio: 12, attack: 0.003, release: 0.25 }
    for (const [key, val] of Object.entries(defaults)) {
      localStorage.setItem(`akroasis_compressor_${key}`, JSON.stringify(val))
    }
    localStorage.setItem('akroasis_compressor_enabled', 'false')
    set({ ...defaults, enabled: false, activePreset: null })
  },
}))

export { BUILT_IN_PRESETS as COMPRESSOR_PRESETS }
