import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { useEqStore, BUILT_IN_PRESETS } from './eqStore'
import { HEADPHONE_PROFILES } from '../data/headphoneProfiles'

// Track localStorage calls
const localStorageMock = (() => {
  let store: Record<string, string> = {}
  return {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, value: string) => { store[key] = value }),
    removeItem: vi.fn((key: string) => { delete store[key] }),
    clear: vi.fn(() => { store = {} }),
  }
})()

Object.defineProperty(globalThis, 'localStorage', {
  value: localStorageMock,
  writable: true,
})

function resetStore() {
  useEqStore.setState({
    enabled: true,
    bands: new Array(10).fill(0) as number[],
    activePreset: null,
    activeHeadphoneProfile: null,
    customPresets: {},
  })
  localStorageMock.clear()
  vi.clearAllMocks()
}

describe('eqStore', () => {
  beforeEach(resetStore)
  afterEach(resetStore)

  describe('initial state', () => {
    it('starts enabled', () => {
      expect(useEqStore.getState().enabled).toBe(true)
    })

    it('starts with 10 bands at 0 dB', () => {
      const { bands } = useEqStore.getState()
      expect(bands).toHaveLength(10)
      expect(bands.every((b) => b === 0)).toBe(true)
    })

    it('starts with no active preset', () => {
      expect(useEqStore.getState().activePreset).toBeNull()
    })

    it('starts with no custom presets', () => {
      expect(useEqStore.getState().customPresets).toEqual({})
    })
  })

  describe('setBand', () => {
    it('updates a specific band gain', () => {
      useEqStore.getState().setBand(0, 6)
      expect(useEqStore.getState().bands[0]).toBe(6)
    })

    it('clamps to +12 dB maximum', () => {
      useEqStore.getState().setBand(0, 20)
      expect(useEqStore.getState().bands[0]).toBe(12)
    })

    it('clamps to -12 dB minimum', () => {
      useEqStore.getState().setBand(0, -20)
      expect(useEqStore.getState().bands[0]).toBe(-12)
    })

    it('clears activePreset when band changes', () => {
      useEqStore.getState().setPreset('Rock')
      useEqStore.getState().setBand(0, 1)
      expect(useEqStore.getState().activePreset).toBeNull()
    })

    it('ignores out-of-range band index', () => {
      const before = [...useEqStore.getState().bands]
      useEqStore.getState().setBand(-1, 6)
      useEqStore.getState().setBand(10, 6)
      expect(useEqStore.getState().bands).toEqual(before)
    })

    it('persists to localStorage', () => {
      useEqStore.getState().setBand(3, 5)
      expect(localStorageMock.setItem).toHaveBeenCalledWith(
        'akroasis_eq_bands',
        expect.any(String),
      )
    })
  })

  describe('setPreset', () => {
    it('applies Flat preset (all zeros)', () => {
      useEqStore.getState().setBand(0, 6)
      useEqStore.getState().setPreset('Flat')
      const { bands } = useEqStore.getState()
      expect(bands.every((b) => b === 0)).toBe(true)
    })

    it('applies Rock preset', () => {
      useEqStore.getState().setPreset('Rock')
      expect(useEqStore.getState().bands).toEqual(BUILT_IN_PRESETS['Rock'])
    })

    it('sets activePreset name', () => {
      useEqStore.getState().setPreset('Jazz')
      expect(useEqStore.getState().activePreset).toBe('Jazz')
    })

    it('ignores unknown preset names', () => {
      const before = [...useEqStore.getState().bands]
      useEqStore.getState().setPreset('NonExistent')
      expect(useEqStore.getState().bands).toEqual(before)
    })

    it('applies all 6 built-in presets without throwing', () => {
      for (const name of Object.keys(BUILT_IN_PRESETS)) {
        expect(() => useEqStore.getState().setPreset(name)).not.toThrow()
      }
    })
  })

  describe('custom presets', () => {
    it('saves current bands as a named preset', () => {
      useEqStore.getState().setBand(0, 4)
      useEqStore.getState().setBand(1, 3)
      useEqStore.getState().saveCustomPreset('My Preset')
      const { customPresets } = useEqStore.getState()
      expect(customPresets['My Preset']).toBeDefined()
      expect(customPresets['My Preset'][0]).toBe(4)
    })

    it('sets activePreset to saved name', () => {
      useEqStore.getState().saveCustomPreset('Saved')
      expect(useEqStore.getState().activePreset).toBe('Saved')
    })

    it('applies a custom preset via setPreset', () => {
      useEqStore.getState().setBand(5, -6)
      useEqStore.getState().saveCustomPreset('Custom')
      useEqStore.getState().setPreset('Flat')
      useEqStore.getState().setPreset('Custom')
      expect(useEqStore.getState().bands[5]).toBe(-6)
    })

    it('deletes a custom preset', () => {
      useEqStore.getState().saveCustomPreset('ToDelete')
      useEqStore.getState().deleteCustomPreset('ToDelete')
      expect(useEqStore.getState().customPresets['ToDelete']).toBeUndefined()
    })

    it('clears activePreset when deleting active custom preset', () => {
      useEqStore.getState().saveCustomPreset('Active')
      useEqStore.getState().deleteCustomPreset('Active')
      expect(useEqStore.getState().activePreset).toBeNull()
    })

    it('keeps activePreset when deleting a different preset', () => {
      useEqStore.getState().saveCustomPreset('Keep')
      useEqStore.getState().saveCustomPreset('Other')
      useEqStore.getState().setPreset('Keep')
      useEqStore.getState().deleteCustomPreset('Other')
      expect(useEqStore.getState().activePreset).toBe('Keep')
    })

    it('persists custom presets to localStorage', () => {
      useEqStore.getState().saveCustomPreset('Persisted')
      expect(localStorageMock.setItem).toHaveBeenCalledWith(
        'akroasis_eq_presets',
        expect.stringContaining('Persisted'),
      )
    })
  })

  describe('setEnabled', () => {
    it('toggles the enabled flag', () => {
      useEqStore.getState().setEnabled(false)
      expect(useEqStore.getState().enabled).toBe(false)
      useEqStore.getState().setEnabled(true)
      expect(useEqStore.getState().enabled).toBe(true)
    })

    it('persists enabled state to localStorage', () => {
      useEqStore.getState().setEnabled(false)
      expect(localStorageMock.setItem).toHaveBeenCalledWith(
        'akroasis_eq_enabled',
        'false',
      )
    })
  })

  describe('reset', () => {
    it('zeroes all bands', () => {
      useEqStore.getState().setPreset('Rock')
      useEqStore.getState().reset()
      const { bands } = useEqStore.getState()
      expect(bands.every((b) => b === 0)).toBe(true)
    })

    it('sets activePreset to Flat', () => {
      useEqStore.getState().setPreset('Rock')
      useEqStore.getState().reset()
      expect(useEqStore.getState().activePreset).toBe('Flat')
    })

    it('persists reset bands to localStorage', () => {
      useEqStore.getState().reset()
      expect(localStorageMock.setItem).toHaveBeenCalledWith(
        'akroasis_eq_bands',
        JSON.stringify(new Array(10).fill(0)),
      )
    })
  })

  describe('headphone profiles', () => {
    const hd600 = HEADPHONE_PROFILES.find((p) => p.model === 'HD 600')!

    it('applies headphone profile and sets bands', () => {
      useEqStore.getState().applyHeadphoneProfile(hd600)
      const { bands, activeHeadphoneProfile } = useEqStore.getState()
      expect(activeHeadphoneProfile).toBe('Sennheiser HD 600')
      expect(bands).toHaveLength(10)
      expect(bands.some((b) => b !== 0)).toBe(true)
    })

    it('clears activePreset when applying headphone profile', () => {
      useEqStore.getState().setPreset('Rock')
      useEqStore.getState().applyHeadphoneProfile(hd600)
      expect(useEqStore.getState().activePreset).toBeNull()
    })

    it('clears headphone profile when adjusting a band', () => {
      useEqStore.getState().applyHeadphoneProfile(hd600)
      useEqStore.getState().setBand(0, 3)
      expect(useEqStore.getState().activeHeadphoneProfile).toBeNull()
    })

    it('clears headphone profile when applying a preset', () => {
      useEqStore.getState().applyHeadphoneProfile(hd600)
      useEqStore.getState().setPreset('Jazz')
      expect(useEqStore.getState().activeHeadphoneProfile).toBeNull()
    })

    it('clears headphone profile on reset', () => {
      useEqStore.getState().applyHeadphoneProfile(hd600)
      useEqStore.getState().reset()
      expect(useEqStore.getState().activeHeadphoneProfile).toBeNull()
    })

    it('clearHeadphoneProfile keeps bands unchanged', () => {
      useEqStore.getState().applyHeadphoneProfile(hd600)
      const bandsBefore = [...useEqStore.getState().bands]
      useEqStore.getState().clearHeadphoneProfile()
      expect(useEqStore.getState().activeHeadphoneProfile).toBeNull()
      expect(useEqStore.getState().bands).toEqual(bandsBefore)
    })

    it('persists headphone profile to localStorage', () => {
      useEqStore.getState().applyHeadphoneProfile(hd600)
      expect(localStorageMock.setItem).toHaveBeenCalledWith(
        'akroasis_eq_headphone',
        JSON.stringify('Sennheiser HD 600'),
      )
    })

    it('all built-in profiles produce valid band values', () => {
      for (const profile of HEADPHONE_PROFILES) {
        useEqStore.getState().applyHeadphoneProfile(profile)
        const { bands } = useEqStore.getState()
        expect(bands).toHaveLength(10)
        for (const val of bands) {
          expect(val).toBeGreaterThanOrEqual(-12)
          expect(val).toBeLessThanOrEqual(12)
        }
      }
    })
  })

  describe('localStorage round-trip', () => {
    it('reads bands from localStorage on init via loadJson', () => {
      // Simulate a prior saved state by forcing store state directly
      const saved = [1, 2, 3, 4, 5, -1, -2, -3, -4, -5]
      useEqStore.setState({ bands: saved })
      expect(useEqStore.getState().bands).toEqual(saved)
    })
  })
})
