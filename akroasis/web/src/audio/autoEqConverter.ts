// Converts AutoEQ parametric profiles to 10-band fixed EQ settings
import type { HeadphoneProfile, ParametricBand } from '../types'

const ISO_FREQUENCIES = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]
const GAIN_MIN = -12
const GAIN_MAX = 12

export function convertToFixedBands(profile: HeadphoneProfile): number[] {
  return ISO_FREQUENCIES.map((centerFreq) => {
    let totalGain = 0
    for (const band of profile.parametricEq) {
      totalGain += calculateGainContribution(band, centerFreq)
    }
    const clamped = Math.max(GAIN_MIN, Math.min(GAIN_MAX, totalGain))
    return Math.round(clamped * 2) / 2
  })
}

function calculateGainContribution(band: ParametricBand, frequency: number): number {
  switch (band.type) {
    case 'peaking':
      return calculatePeakingGain(band, frequency)
    case 'low_shelf':
      return calculateLowShelfGain(band, frequency)
    case 'high_shelf':
      return calculateHighShelfGain(band, frequency)
    case 'low_pass':
    case 'high_pass':
      return 0
  }
}

function calculatePeakingGain(band: ParametricBand, frequency: number): number {
  const w0 = 2 * Math.PI * band.frequency
  const w = 2 * Math.PI * frequency
  const distance = Math.abs(Math.log(w / w0))
  const bandwidth = Math.log(2) / (2 * band.q)

  if (distance < bandwidth * 3) {
    const attenuation = Math.exp(-distance / bandwidth)
    return band.gain * attenuation
  }
  return 0
}

function calculateLowShelfGain(band: ParametricBand, frequency: number): number {
  if (frequency >= band.frequency) {
    return band.gain
  }
  const ratio = frequency / band.frequency
  const slope = Math.max(0.1, Math.min(1.0, band.q))
  const attenuation = Math.max(0, Math.min(1, Math.pow(ratio, slope * 2)))
  return band.gain * attenuation
}

function calculateHighShelfGain(band: ParametricBand, frequency: number): number {
  if (frequency <= band.frequency) {
    return band.gain
  }
  const ratio = band.frequency / frequency
  const slope = Math.max(0.1, Math.min(1.0, band.q))
  const attenuation = Math.max(0, Math.min(1, Math.pow(ratio, slope * 2)))
  return band.gain * attenuation
}

export function searchProfiles(profiles: HeadphoneProfile[], query: string): HeadphoneProfile[] {
  if (!query.trim()) return profiles
  const lower = query.toLowerCase()
  return profiles.filter(
    (p) =>
      `${p.manufacturer} ${p.model}`.toLowerCase().includes(lower) ||
      p.manufacturer.toLowerCase().includes(lower) ||
      p.model.toLowerCase().includes(lower),
  )
}

export function groupByManufacturer(profiles: HeadphoneProfile[]): Map<string, HeadphoneProfile[]> {
  const groups = new Map<string, HeadphoneProfile[]>()
  for (const profile of profiles) {
    const existing = groups.get(profile.manufacturer) ?? []
    existing.push(profile)
    groups.set(profile.manufacturer, existing)
  }
  return groups
}
