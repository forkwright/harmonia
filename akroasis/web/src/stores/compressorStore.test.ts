import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { useCompressorStore } from './compressorStore'

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
  useCompressorStore.setState({
    enabled: false,
    activePreset: null,
    threshold: -24,
    knee: 30,
    ratio: 12,
    attack: 0.003,
    release: 0.25,
  })
  localStorageMock.clear()
  vi.clearAllMocks()
}

describe('compressorStore', () => {
  beforeEach(resetStore)
  afterEach(resetStore)

  describe('initial state', () => {
    it('starts disabled', () => {
      expect(useCompressorStore.getState().enabled).toBe(false)
    })

    it('starts with no active preset', () => {
      expect(useCompressorStore.getState().activePreset).toBeNull()
    })

    it('has default parameter values', () => {
      const state = useCompressorStore.getState()
      expect(state.threshold).toBe(-24)
      expect(state.knee).toBe(30)
      expect(state.ratio).toBe(12)
      expect(state.attack).toBe(0.003)
      expect(state.release).toBe(0.25)
    })
  })

  describe('setEnabled', () => {
    it('toggles enabled state', () => {
      useCompressorStore.getState().setEnabled(true)
      expect(useCompressorStore.getState().enabled).toBe(true)
      useCompressorStore.getState().setEnabled(false)
      expect(useCompressorStore.getState().enabled).toBe(false)
    })

    it('persists to localStorage', () => {
      useCompressorStore.getState().setEnabled(true)
      expect(localStorageMock.setItem).toHaveBeenCalledWith(
        'akroasis_compressor_enabled',
        'true',
      )
    })
  })

  describe('setPreset', () => {
    it('applies Speech preset', () => {
      useCompressorStore.getState().setPreset('Speech')
      const state = useCompressorStore.getState()
      expect(state.threshold).toBe(-20)
      expect(state.knee).toBe(20)
      expect(state.ratio).toBe(8)
      expect(state.attack).toBe(0.005)
      expect(state.release).toBe(0.15)
      expect(state.activePreset).toBe('Speech')
      expect(state.enabled).toBe(true)
    })

    it('applies Night Mode preset', () => {
      useCompressorStore.getState().setPreset('Night Mode')
      const state = useCompressorStore.getState()
      expect(state.threshold).toBe(-30)
      expect(state.ratio).toBe(20)
      expect(state.activePreset).toBe('Night Mode')
    })

    it('applies Gentle preset', () => {
      useCompressorStore.getState().setPreset('Gentle')
      const state = useCompressorStore.getState()
      expect(state.threshold).toBe(-18)
      expect(state.ratio).toBe(4)
    })

    it('ignores unknown preset', () => {
      const before = { ...useCompressorStore.getState() }
      useCompressorStore.getState().setPreset('NonExistent')
      expect(useCompressorStore.getState().threshold).toBe(before.threshold)
    })

    it('auto-enables when applying preset', () => {
      useCompressorStore.getState().setEnabled(false)
      useCompressorStore.getState().setPreset('Speech')
      expect(useCompressorStore.getState().enabled).toBe(true)
    })

    it('persists all params to localStorage', () => {
      useCompressorStore.getState().setPreset('Speech')
      expect(localStorageMock.setItem).toHaveBeenCalledWith(
        'akroasis_compressor_threshold',
        '-20',
      )
    })
  })

  describe('setParam', () => {
    it('updates threshold', () => {
      useCompressorStore.getState().setParam('threshold', -40)
      expect(useCompressorStore.getState().threshold).toBe(-40)
    })

    it('clamps threshold to valid range', () => {
      useCompressorStore.getState().setParam('threshold', -200)
      expect(useCompressorStore.getState().threshold).toBe(-100)
      useCompressorStore.getState().setParam('threshold', 50)
      expect(useCompressorStore.getState().threshold).toBe(0)
    })

    it('clamps ratio to valid range', () => {
      useCompressorStore.getState().setParam('ratio', 0)
      expect(useCompressorStore.getState().ratio).toBe(1)
      useCompressorStore.getState().setParam('ratio', 100)
      expect(useCompressorStore.getState().ratio).toBe(20)
    })

    it('clamps attack and release to valid range', () => {
      useCompressorStore.getState().setParam('attack', -1)
      expect(useCompressorStore.getState().attack).toBe(0)
      useCompressorStore.getState().setParam('release', 5)
      expect(useCompressorStore.getState().release).toBe(1)
    })

    it('clears activePreset when param changes', () => {
      useCompressorStore.getState().setPreset('Speech')
      useCompressorStore.getState().setParam('threshold', -30)
      expect(useCompressorStore.getState().activePreset).toBeNull()
    })

    it('persists param to localStorage', () => {
      useCompressorStore.getState().setParam('knee', 15)
      expect(localStorageMock.setItem).toHaveBeenCalledWith(
        'akroasis_compressor_knee',
        '15',
      )
    })
  })

  describe('reset', () => {
    it('restores all defaults', () => {
      useCompressorStore.getState().setPreset('Night Mode')
      useCompressorStore.getState().reset()
      const state = useCompressorStore.getState()
      expect(state.enabled).toBe(false)
      expect(state.activePreset).toBeNull()
      expect(state.threshold).toBe(-24)
      expect(state.knee).toBe(30)
      expect(state.ratio).toBe(12)
    })

    it('persists reset state to localStorage', () => {
      useCompressorStore.getState().reset()
      expect(localStorageMock.setItem).toHaveBeenCalledWith(
        'akroasis_compressor_enabled',
        'false',
      )
    })
  })
})
