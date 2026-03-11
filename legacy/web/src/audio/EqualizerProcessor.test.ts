import { describe, it, expect, beforeEach, vi } from 'vitest'
import { EqualizerProcessor } from './EqualizerProcessor'

function makeMockFilter() {
  return {
    type: 'peaking' as BiquadFilterType,
    frequency: { value: 0 },
    Q: { value: 0 },
    gain: { value: 0 },
    connect: vi.fn(),
    disconnect: vi.fn(),
  }
}

function makeMockGain() {
  return {
    gain: { value: 1 },
    connect: vi.fn(),
    disconnect: vi.fn(),
  }
}

function makeMockContext(): AudioContext {
  const filters = Array.from({ length: 10 }, makeMockFilter)
  let filterCallCount = 0

  return {
    createBiquadFilter: vi.fn(() => filters[filterCallCount++]),
    createGain: vi.fn(makeMockGain),
    sampleRate: 44100,
    destination: {},
  } as unknown as AudioContext
}

describe('EqualizerProcessor', () => {
  let context: AudioContext
  let eq: EqualizerProcessor

  beforeEach(() => {
    context = makeMockContext()
    eq = new EqualizerProcessor(context)
  })

  describe('construction', () => {
    it('creates 10 BiquadFilterNodes', () => {
      expect(context.createBiquadFilter).toHaveBeenCalledTimes(10)
    })

    it('sets each filter to peaking type', () => {
      const filters = eq.getFilters()
      for (const f of filters) {
        expect(f.type).toBe('peaking')
      }
    })

    it('assigns ISO 10-band center frequencies', () => {
      const expected = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]
      const filters = eq.getFilters()
      for (let i = 0; i < 10; i++) {
        expect(filters[i].frequency.value).toBe(expected[i])
      }
    })

    it('initializes gains to 0', () => {
      const filters = eq.getFilters()
      for (const f of filters) {
        expect(f.gain.value).toBe(0)
      }
    })

    it('creates an input gain node', () => {
      expect(context.createGain).toHaveBeenCalledTimes(1)
      expect(eq.getInputNode()).toBeDefined()
    })

    it('exposes the correct frequencies', () => {
      const freqs = eq.getFrequencies()
      expect(freqs).toEqual([31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000])
    })
  })

  describe('setGain', () => {
    it('sets gain on the correct filter', () => {
      eq.setGain(0, 6)
      expect(eq.getFilters()[0].gain.value).toBe(6)

      eq.setGain(9, -6)
      expect(eq.getFilters()[9].gain.value).toBe(-6)
    })

    it('clamps gain to +12 dB maximum', () => {
      eq.setGain(0, 20)
      expect(eq.getFilters()[0].gain.value).toBe(12)
    })

    it('clamps gain to -12 dB minimum', () => {
      eq.setGain(0, -20)
      expect(eq.getFilters()[0].gain.value).toBe(-12)
    })

    it('ignores out-of-range band index', () => {
      expect(() => eq.setGain(-1, 6)).not.toThrow()
      expect(() => eq.setGain(10, 6)).not.toThrow()
    })
  })

  describe('setAllGains', () => {
    it('applies gains to all 10 bands', () => {
      const gains = [1, 2, 3, 4, 5, -1, -2, -3, -4, -5]
      eq.setAllGains(gains)
      const filters = eq.getFilters()
      for (let i = 0; i < 10; i++) {
        expect(filters[i].gain.value).toBe(gains[i])
      }
    })

    it('clamps values outside -12/+12 range', () => {
      eq.setAllGains([15, -15, 0, 0, 0, 0, 0, 0, 0, 0])
      expect(eq.getFilters()[0].gain.value).toBe(12)
      expect(eq.getFilters()[1].gain.value).toBe(-12)
    })

    it('defaults missing entries to 0', () => {
      eq.setAllGains([3])
      expect(eq.getFilters()[0].gain.value).toBe(3)
      expect(eq.getFilters()[5].gain.value).toBe(0)
    })
  })

  describe('setEnabled', () => {
    it('zeroes all gains when disabled', () => {
      eq.setAllGains([6, 6, 6, 6, 6, 6, 6, 6, 6, 6])
      eq.setEnabled(false)
      const filters = eq.getFilters()
      for (const f of filters) {
        expect(f.gain.value).toBe(0)
      }
    })

    it('blocks setGain from applying when disabled', () => {
      eq.setEnabled(false)
      eq.setGain(0, 6)
      expect(eq.getFilters()[0].gain.value).toBe(0)
    })

    it('allows setGain to apply when re-enabled', () => {
      eq.setEnabled(false)
      eq.setEnabled(true)
      eq.setGain(0, 6)
      expect(eq.getFilters()[0].gain.value).toBe(6)
    })
  })

  describe('connect / disconnect', () => {
    it('connects last filter to output node', () => {
      const output = makeMockGain() as unknown as AudioNode
      eq.connect(output)
      const lastFilter = eq.getFilters()[9]
      expect(lastFilter.connect).toHaveBeenCalledWith(output)
    })

    it('disconnects without throwing', () => {
      const output = makeMockGain() as unknown as AudioNode
      eq.connect(output)
      expect(() => eq.disconnect()).not.toThrow()
    })
  })

  describe('static properties', () => {
    it('exposes bandCount as 10', () => {
      expect(EqualizerProcessor.bandCount).toBe(10)
    })

    it('exposes gainMin as -12', () => {
      expect(EqualizerProcessor.gainMin).toBe(-12)
    })

    it('exposes gainMax as +12', () => {
      expect(EqualizerProcessor.gainMax).toBe(12)
    })
  })
})
