// replayGainStore — ReplayGain store tests
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { useReplayGainStore } from './replayGainStore'

// Mock localStorage
const storage = new Map<string, string>()
vi.stubGlobal('localStorage', {
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, val: string) => storage.set(key, val),
  removeItem: (key: string) => storage.delete(key),
  clear: () => storage.clear(),
})

function resetStore() {
  storage.clear()
  useReplayGainStore.setState({
    mode: 'off',
    targetLufs: -18,
    limiterEnabled: true,
    preScanEnabled: true,
    analysisCache: {},
  })
}

describe('replayGainStore', () => {
  beforeEach(resetStore)

  it('defaults to off mode with -18 LUFS target', () => {
    const state = useReplayGainStore.getState()
    expect(state.mode).toBe('off')
    expect(state.targetLufs).toBe(-18)
    expect(state.limiterEnabled).toBe(true)
    expect(state.preScanEnabled).toBe(true)
  })

  it('changes mode and persists', () => {
    useReplayGainStore.getState().setMode('track')
    expect(useReplayGainStore.getState().mode).toBe('track')
    const stored = JSON.parse(storage.get('akroasis_replayGain')!)
    expect(stored.mode).toBe('track')
  })

  it('clamps target LUFS to -23..-14', () => {
    useReplayGainStore.getState().setTargetLufs(-30)
    expect(useReplayGainStore.getState().targetLufs).toBe(-23)

    useReplayGainStore.getState().setTargetLufs(-10)
    expect(useReplayGainStore.getState().targetLufs).toBe(-14)

    useReplayGainStore.getState().setTargetLufs(-20)
    expect(useReplayGainStore.getState().targetLufs).toBe(-20)
  })

  it('toggles limiter and pre-scan', () => {
    useReplayGainStore.getState().setLimiterEnabled(false)
    expect(useReplayGainStore.getState().limiterEnabled).toBe(false)

    useReplayGainStore.getState().setPreScanEnabled(false)
    expect(useReplayGainStore.getState().preScanEnabled).toBe(false)
  })

  describe('getEffectiveGain', () => {
    const baseTrack = {
      id: 1, title: 'Test', artist: 'A', album: 'B',
      duration: 180, fileSize: 1000, format: 'flac',
      bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2,
    }

    it('returns null when mode is off', () => {
      const gain = useReplayGainStore.getState().getEffectiveGain(baseTrack)
      expect(gain).toBeNull()
    })

    it('uses R128 track gain when available in track mode', () => {
      useReplayGainStore.getState().setMode('track')
      // R128 gain: 256 units = 1 LU. If gain is 256 (1 LU), measured = -23 + 1 = -22 LUFS
      // target -18, so adjustment = -18 - (-22) = 4 dB
      const track = { ...baseTrack, r128TrackGain: 256 }
      const gain = useReplayGainStore.getState().getEffectiveGain(track)
      expect(gain).toBe(4)
    })

    it('uses R128 album gain in album mode', () => {
      useReplayGainStore.getState().setMode('album')
      const track = { ...baseTrack, r128AlbumGain: 0 }
      // gain = 0 → measured = -23 LUFS, target = -18, adjustment = -18 - (-23) = 5
      const gain = useReplayGainStore.getState().getEffectiveGain(track)
      expect(gain).toBe(5)
    })

    it('falls back to ReplayGain tags when no R128', () => {
      useReplayGainStore.getState().setMode('track')
      // RG track gain: -6dB means measured = -18 - (-6) = -12 LUFS
      // target -18, adjustment = -18 - (-12) = -6
      const track = { ...baseTrack, replayGainTrackGain: -6 }
      const gain = useReplayGainStore.getState().getEffectiveGain(track)
      expect(gain).toBe(-6)
    })

    it('falls back to analysis cache', () => {
      useReplayGainStore.getState().setMode('track')
      useReplayGainStore.setState({
        analysisCache: { 1: -14 }, // measured -14 LUFS
      })
      // computeReplayGain(-14, -18) = -18 - (-14) = -4
      const gain = useReplayGainStore.getState().getEffectiveGain(baseTrack)
      expect(gain).toBe(-4)
    })

    it('returns null when no data available', () => {
      useReplayGainStore.getState().setMode('track')
      const gain = useReplayGainStore.getState().getEffectiveGain(baseTrack)
      expect(gain).toBeNull()
    })
  })
})
