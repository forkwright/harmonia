// Crossfade / Metaxis — the space between tracks
import { create } from 'zustand'
import { loadJson, saveJson } from '../utils/storage'

export type CrossfadeMode = 'off' | 'simple'
export type CrossfadeCurve = 'linear' | 'equalPower' | 'sCurve'

interface MetaxisState {
  mode: CrossfadeMode
  duration: number
  curve: CrossfadeCurve
  respectAlbumTransitions: boolean

  setMode: (mode: CrossfadeMode) => void
  setDuration: (seconds: number) => void
  setCurve: (curve: CrossfadeCurve) => void
  setRespectAlbumTransitions: (respect: boolean) => void
  shouldCrossfade: (currentAlbum: string | undefined, nextAlbum: string | undefined) => boolean
}

const STORAGE_KEY = 'akroasis_metaxis'

function loadState() {
  return loadJson(STORAGE_KEY, {
    mode: 'off' as CrossfadeMode,
    duration: 3,
    curve: 'equalPower' as CrossfadeCurve,
    respectAlbumTransitions: true,
  })
}

function persist(partial: Partial<{ mode: CrossfadeMode; duration: number; curve: CrossfadeCurve; respectAlbumTransitions: boolean }>) {
  const current = loadJson(STORAGE_KEY, {})
  saveJson(STORAGE_KEY, { ...current, ...partial })
}

export const useMetaxisStore = create<MetaxisState>((set, get) => {
  const initial = loadState()
  return {
    mode: initial.mode,
    duration: initial.duration,
    curve: initial.curve,
    respectAlbumTransitions: initial.respectAlbumTransitions,

    setMode: (mode) => {
      set({ mode })
      persist({ mode })
    },

    setDuration: (seconds) => {
      const clamped = Math.max(0, Math.min(12, seconds))
      set({ duration: clamped })
      persist({ duration: clamped })
    },

    setCurve: (curve) => {
      set({ curve })
      persist({ curve })
    },

    setRespectAlbumTransitions: (respect) => {
      set({ respectAlbumTransitions: respect })
      persist({ respectAlbumTransitions: respect })
    },

    shouldCrossfade: (currentAlbum, nextAlbum) => {
      const { mode, respectAlbumTransitions } = get()
      if (mode === 'off') return false
      if (respectAlbumTransitions && currentAlbum && nextAlbum && currentAlbum === nextAlbum) return false
      return true
    },
  }
})
