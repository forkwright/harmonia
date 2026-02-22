// loudnessMeasure — LUFS measurement tests
import { describe, it, expect, vi } from 'vitest'
import { measureLoudness, computeReplayGain, dbToLinear } from './loudnessMeasure'

function createMockBuffer(samples: Float32Array, sampleRate = 44100): AudioBuffer {
  return {
    numberOfChannels: 1,
    sampleRate,
    length: samples.length,
    duration: samples.length / sampleRate,
    getChannelData: vi.fn(() => samples),
    copyFromChannel: vi.fn(),
    copyToChannel: vi.fn(),
  } as unknown as AudioBuffer
}

function createStereoMockBuffer(left: Float32Array, right: Float32Array, sampleRate = 44100): AudioBuffer {
  return {
    numberOfChannels: 2,
    sampleRate,
    length: left.length,
    duration: left.length / sampleRate,
    getChannelData: vi.fn((ch: number) => ch === 0 ? left : right),
    copyFromChannel: vi.fn(),
    copyToChannel: vi.fn(),
  } as unknown as AudioBuffer
}

describe('measureLoudness', () => {
  it('returns -70 LUFS for silence', () => {
    const silence = new Float32Array(44100) // 1 second of silence
    const buffer = createMockBuffer(silence)
    const result = measureLoudness(buffer)
    expect(result.integratedLufs).toBe(-70)
    expect(result.peakLinear).toBe(0)
  })

  it('returns -70 for empty buffer', () => {
    const empty = new Float32Array(0)
    const buffer = createMockBuffer(empty)
    const result = measureLoudness(buffer)
    expect(result.integratedLufs).toBe(-70)
  })

  it('measures loudness for a sine wave', () => {
    // Generate 1 second of 1kHz sine at 0.5 amplitude
    const sampleRate = 44100
    const samples = new Float32Array(sampleRate)
    for (let i = 0; i < sampleRate; i++) {
      samples[i] = 0.5 * Math.sin(2 * Math.PI * 1000 * i / sampleRate)
    }
    const buffer = createMockBuffer(samples, sampleRate)
    const result = measureLoudness(buffer)

    // 0.5 amplitude sine → RMS ≈ 0.354, with K-weighting (+4dB ≈ 1.585x) → boosted RMS ≈ 0.561
    // Power ≈ 0.315, LUFS ≈ -0.691 + 10*log10(0.315) ≈ -0.691 + (-5.02) ≈ -5.7
    // Should be somewhere in the range of -10 to 0 LUFS (depending on gating)
    expect(result.integratedLufs).toBeGreaterThan(-20)
    expect(result.integratedLufs).toBeLessThan(0)
    expect(result.peakLinear).toBeCloseTo(0.5, 1)
  })

  it('measures stereo buffer', () => {
    const sampleRate = 44100
    const left = new Float32Array(sampleRate)
    const right = new Float32Array(sampleRate)
    for (let i = 0; i < sampleRate; i++) {
      left[i] = 0.3 * Math.sin(2 * Math.PI * 440 * i / sampleRate)
      right[i] = 0.3 * Math.sin(2 * Math.PI * 440 * i / sampleRate)
    }
    const buffer = createStereoMockBuffer(left, right, sampleRate)
    const result = measureLoudness(buffer)

    expect(result.integratedLufs).toBeGreaterThan(-30)
    expect(result.integratedLufs).toBeLessThan(0)
    expect(result.peakLinear).toBeCloseTo(0.3, 1)
  })

  it('louder signal measures higher LUFS', () => {
    const sampleRate = 44100
    const quiet = new Float32Array(sampleRate)
    const loud = new Float32Array(sampleRate)
    for (let i = 0; i < sampleRate; i++) {
      quiet[i] = 0.1 * Math.sin(2 * Math.PI * 1000 * i / sampleRate)
      loud[i] = 0.8 * Math.sin(2 * Math.PI * 1000 * i / sampleRate)
    }
    const quietResult = measureLoudness(createMockBuffer(quiet, sampleRate))
    const loudResult = measureLoudness(createMockBuffer(loud, sampleRate))

    expect(loudResult.integratedLufs).toBeGreaterThan(quietResult.integratedLufs)
  })
})

describe('computeReplayGain', () => {
  it('returns positive gain for quiet tracks', () => {
    expect(computeReplayGain(-24, -18)).toBe(6)
  })

  it('returns negative gain for loud tracks', () => {
    expect(computeReplayGain(-12, -18)).toBe(-6)
  })

  it('returns zero when at target', () => {
    expect(computeReplayGain(-18, -18)).toBe(0)
  })
})

describe('dbToLinear', () => {
  it('converts 0 dB to 1.0', () => {
    expect(dbToLinear(0)).toBeCloseTo(1.0)
  })

  it('converts +6 dB to ~2.0', () => {
    expect(dbToLinear(6)).toBeCloseTo(1.995, 2)
  })

  it('converts -6 dB to ~0.5', () => {
    expect(dbToLinear(-6)).toBeCloseTo(0.501, 2)
  })

  it('converts -20 dB to 0.1', () => {
    expect(dbToLinear(-20)).toBeCloseTo(0.1, 2)
  })
})
