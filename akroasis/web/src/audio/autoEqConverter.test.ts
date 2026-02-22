import { describe, it, expect } from 'vitest'
import { convertToFixedBands, searchProfiles, groupByManufacturer } from './autoEqConverter'
import { HEADPHONE_PROFILES } from '../data/headphoneProfiles'
import type { HeadphoneProfile } from '../types'

describe('autoEqConverter', () => {
  describe('convertToFixedBands', () => {
    it('returns exactly 10 bands', () => {
      const bands = convertToFixedBands(HEADPHONE_PROFILES[0])
      expect(bands).toHaveLength(10)
    })

    it('all values within -12 to +12 dB', () => {
      for (const profile of HEADPHONE_PROFILES) {
        const bands = convertToFixedBands(profile)
        for (const val of bands) {
          expect(val).toBeGreaterThanOrEqual(-12)
          expect(val).toBeLessThanOrEqual(12)
        }
      }
    })

    it('values snap to 0.5 dB increments', () => {
      for (const profile of HEADPHONE_PROFILES.slice(0, 5)) {
        const bands = convertToFixedBands(profile)
        for (const val of bands) {
          expect(val * 2).toBe(Math.round(val * 2))
        }
      }
    })

    it('empty parametric EQ produces all zeros', () => {
      const empty: HeadphoneProfile = { manufacturer: 'Test', model: 'Empty', parametricEq: [] }
      const bands = convertToFixedBands(empty)
      expect(bands.every((b) => b === 0)).toBe(true)
    })

    it('peaking filter concentrates gain near center frequency', () => {
      const profile: HeadphoneProfile = {
        manufacturer: 'Test',
        model: 'Peak1k',
        parametricEq: [{ type: 'peaking', frequency: 1000, gain: 6, q: 2 }],
      }
      const bands = convertToFixedBands(profile)
      const idx1k = 5 // 1000 Hz is index 5
      expect(bands[idx1k]).toBeGreaterThan(0)
      expect(bands[idx1k]).toBeGreaterThan(bands[0])
      expect(bands[idx1k]).toBeGreaterThan(bands[9])
    })

    it('low shelf applies gain at and above transition frequency', () => {
      const profile: HeadphoneProfile = {
        manufacturer: 'Test',
        model: 'LowShelf',
        parametricEq: [{ type: 'low_shelf', frequency: 200, gain: 5, q: 0.7 }],
      }
      const bands = convertToFixedBands(profile)
      // Frequencies above 200 Hz should get full gain
      expect(bands[4]).toBeCloseTo(5, 0) // 500 Hz
      expect(bands[5]).toBeCloseTo(5, 0) // 1000 Hz
    })

    it('high shelf applies gain at and below transition frequency', () => {
      const profile: HeadphoneProfile = {
        manufacturer: 'Test',
        model: 'HighShelf',
        parametricEq: [{ type: 'high_shelf', frequency: 4000, gain: -3, q: 0.7 }],
      }
      const bands = convertToFixedBands(profile)
      // Frequencies below 4000 Hz should get full gain
      expect(bands[3]).toBeCloseTo(-3, 0) // 250 Hz
      // Frequencies above should be attenuated toward 0
      expect(Math.abs(bands[9])).toBeLessThan(Math.abs(bands[5]))
    })

    it('HD 600 produces non-trivial values', () => {
      const hd600 = HEADPHONE_PROFILES.find((p) => p.model === 'HD 600')!
      const bands = convertToFixedBands(hd600)
      const nonZero = bands.filter((b) => b !== 0)
      expect(nonZero.length).toBeGreaterThan(3)
    })

    it('low_pass and high_pass contribute zero gain', () => {
      const profile: HeadphoneProfile = {
        manufacturer: 'Test',
        model: 'PassFilters',
        parametricEq: [
          { type: 'low_pass', frequency: 1000, gain: 6, q: 1 },
          { type: 'high_pass', frequency: 100, gain: 6, q: 1 },
        ],
      }
      const bands = convertToFixedBands(profile)
      expect(bands.every((b) => b === 0)).toBe(true)
    })
  })

  describe('searchProfiles', () => {
    it('returns all profiles for empty query', () => {
      const results = searchProfiles(HEADPHONE_PROFILES, '')
      expect(results).toHaveLength(HEADPHONE_PROFILES.length)
    })

    it('finds by manufacturer (case-insensitive)', () => {
      const results = searchProfiles(HEADPHONE_PROFILES, 'sennheiser')
      expect(results.length).toBeGreaterThan(0)
      expect(results.every((p) => p.manufacturer === 'Sennheiser')).toBe(true)
    })

    it('finds by model (case-insensitive)', () => {
      const results = searchProfiles(HEADPHONE_PROFILES, 'hd 600')
      expect(results).toHaveLength(1)
      expect(results[0].model).toBe('HD 600')
    })

    it('finds by partial match', () => {
      const results = searchProfiles(HEADPHONE_PROFILES, 'dt')
      expect(results.length).toBeGreaterThan(0)
      expect(results.every((p) => p.manufacturer === 'Beyerdynamic')).toBe(true)
    })

    it('returns empty for no match', () => {
      const results = searchProfiles(HEADPHONE_PROFILES, 'nonexistent12345')
      expect(results).toHaveLength(0)
    })
  })

  describe('groupByManufacturer', () => {
    it('groups profiles correctly', () => {
      const groups = groupByManufacturer(HEADPHONE_PROFILES)
      expect(groups.has('Sennheiser')).toBe(true)
      expect(groups.has('Beyerdynamic')).toBe(true)
      const sennheiser = groups.get('Sennheiser')!
      expect(sennheiser.length).toBeGreaterThan(1)
      expect(sennheiser.every((p) => p.manufacturer === 'Sennheiser')).toBe(true)
    })

    it('total profiles across groups equals input', () => {
      const groups = groupByManufacturer(HEADPHONE_PROFILES)
      let total = 0
      for (const profiles of groups.values()) {
        total += profiles.length
      }
      expect(total).toBe(HEADPHONE_PROFILES.length)
    })
  })
})
