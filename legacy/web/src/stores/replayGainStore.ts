// ReplayGain / Isosthenia — loudness normalization store
import { create } from 'zustand'
import { loadJson, saveJson } from '../utils/storage'
import type { Track } from '../types'
import { measureLoudness, computeReplayGain, dbToLinear } from '../audio/loudnessMeasure'

export type ReplayGainMode = 'off' | 'track' | 'album'

interface IsostheniaState {
  mode: ReplayGainMode
  targetLufs: number
  limiterEnabled: boolean
  preScanEnabled: boolean
  analysisCache: Record<number, number>

  setMode: (mode: ReplayGainMode) => void
  setTargetLufs: (lufs: number) => void
  setLimiterEnabled: (enabled: boolean) => void
  setPreScanEnabled: (enabled: boolean) => void
  getEffectiveGain: (track: Track) => number | null
  analyzeBuffer: (trackId: number, buffer: AudioBuffer) => number
}

const STORAGE_KEY = 'akroasis_replayGain'

function loadState() {
  return loadJson(STORAGE_KEY, {
    mode: 'off' as ReplayGainMode,
    targetLufs: -18,
    limiterEnabled: true,
    preScanEnabled: true,
  })
}

function persist(partial: Partial<{ mode: ReplayGainMode; targetLufs: number; limiterEnabled: boolean; preScanEnabled: boolean }>) {
  const current = loadJson(STORAGE_KEY, {})
  saveJson(STORAGE_KEY, { ...current, ...partial })
}

export const useReplayGainStore = create<IsostheniaState>((set, get) => {
  const initial = loadState()
  return {
    mode: initial.mode,
    targetLufs: initial.targetLufs,
    limiterEnabled: initial.limiterEnabled,
    preScanEnabled: initial.preScanEnabled,
    analysisCache: {},

    setMode: (mode) => {
      set({ mode })
      persist({ mode })
    },

    setTargetLufs: (lufs) => {
      const clamped = Math.max(-23, Math.min(-14, lufs))
      set({ targetLufs: clamped })
      persist({ targetLufs: clamped })
    },

    setLimiterEnabled: (enabled) => {
      set({ limiterEnabled: enabled })
      persist({ limiterEnabled: enabled })
    },

    setPreScanEnabled: (enabled) => {
      set({ preScanEnabled: enabled })
      persist({ preScanEnabled: enabled })
    },

    // Returns gain in dB, or null if no data available
    getEffectiveGain: (track) => {
      const { mode, targetLufs, analysisCache } = get()
      if (mode === 'off') return null

      // Priority 1: R128 tags (stored in LU relative to -23 LUFS)
      if (mode === 'track' && track.r128TrackGain !== undefined) {
        // R128 gain is in LU: target - (-23 + gain) = target + 23 - gain
        return targetLufs - (-23 + track.r128TrackGain / 256)
      }
      if (mode === 'album' && track.r128AlbumGain !== undefined) {
        return targetLufs - (-23 + track.r128AlbumGain / 256)
      }

      // Priority 2: ReplayGain tags (stored in dB relative to 89 dB SPL ≈ -18 LUFS)
      if (mode === 'track' && track.replayGainTrackGain !== undefined) {
        // RG reference is -18 LUFS: measured = -18 - gain, target adjustment = target - measured
        return targetLufs - (-18 - track.replayGainTrackGain)
      }
      if (mode === 'album' && track.replayGainAlbumGain !== undefined) {
        return targetLufs - (-18 - track.replayGainAlbumGain)
      }

      // Priority 3: Analysis cache
      if (analysisCache[track.id] !== undefined) {
        return computeReplayGain(analysisCache[track.id], targetLufs)
      }

      return null
    },

    // Analyze an AudioBuffer and cache the result. Returns gain in dB.
    analyzeBuffer: (trackId, buffer) => {
      const { targetLufs } = get()
      const { integratedLufs } = measureLoudness(buffer)
      set((state) => ({
        analysisCache: { ...state.analysisCache, [trackId]: integratedLufs },
      }))
      return computeReplayGain(integratedLufs, targetLufs)
    },
  }
})

export { dbToLinear }
