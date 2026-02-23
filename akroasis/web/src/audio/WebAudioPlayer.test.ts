import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { WebAudioPlayer } from './WebAudioPlayer'

// Mock HTMLAudioElement
const mockAudioElement = {
  play: vi.fn().mockResolvedValue(undefined),
  pause: vi.fn(),
  load: vi.fn(),
  removeAttribute: vi.fn(),
  currentTime: 0,
  duration: 180,
  playbackRate: 1,
  volume: 1,
  preload: '',
  src: '',
  onended: null as (() => void) | null,
  onerror: null as (() => void) | null,
  error: null as { code: number; message: string } | null,
}

globalThis.Audio = function() { return mockAudioElement } as unknown as typeof Audio

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {}
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => { store[key] = value },
    removeItem: (key: string) => { delete store[key] },
    clear: () => { store = {} },
  }
})()
Object.defineProperty(globalThis, 'localStorage', { value: localStorageMock })

describe('WebAudioPlayer', () => {
  let player: WebAudioPlayer

  beforeEach(() => {
    vi.clearAllMocks()
    localStorageMock.clear()
    mockAudioElement.currentTime = 0
    mockAudioElement.duration = 180
    mockAudioElement.volume = 1
    mockAudioElement.playbackRate = 1
    mockAudioElement.src = ''
    mockAudioElement.onended = null
    mockAudioElement.onerror = null
    mockAudioElement.error = null
    player = new WebAudioPlayer()
  })

  afterEach(async () => {
    await player.close()
  })

  describe('Playback', () => {
    it('should load and play track', async () => {
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      expect(player.getPlaybackState()).toBe(true)
      expect(mockAudioElement.play).toHaveBeenCalled()
    })

    it('should pause', async () => {
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      player.pause()
      expect(mockAudioElement.pause).toHaveBeenCalled()
      expect(player.getPlaybackState()).toBe(false)
    })

    it('should resume after pause', async () => {
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      player.pause()
      player.play()
      expect(player.getPlaybackState()).toBe(true)
    })

    it('should stop', async () => {
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      player.stop()
      expect(player.getPlaybackState()).toBe(false)
    })

    it('should seek', async () => {
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      player.seek(30)
      expect(mockAudioElement.currentTime).toBe(30)
    })

    it('should replay from beginning', async () => {
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      player.replay()
      expect(mockAudioElement.currentTime).toBe(0)
    })

    it('should set volume directly on audio element', async () => {
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      player.setVolume(0.5)
      expect(mockAudioElement.volume).toBe(0.5)
    })

    it('should set playback speed', async () => {
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      player.setPlaybackSpeed(1.5)
      expect(mockAudioElement.playbackRate).toBe(1.5)
    })
  })

  describe('Auth', () => {
    it('should append token to stream URL', async () => {
      localStorageMock.setItem('accessToken', 'test-jwt')
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      expect(mockAudioElement.src).toContain('token=test-jwt')
    })

    it('should work without token', async () => {
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      expect(mockAudioElement.src).toBe('http://localhost/stream/track/1')
    })
  })

  describe('Preload', () => {
    it('should preload next track without errors', async () => {
      const track1 = { id: 1, title: 'T1' } as any
      const track2 = { id: 2, title: 'T2' } as any
      await player.loadTrack(track1, 'http://localhost/stream/track/1')
      await player.preloadNext(track2, 'http://localhost/stream/track/2')
      // No errors thrown
    })
  })

  describe('Callbacks', () => {
    it('should fire end callback', async () => {
      const endCb = vi.fn()
      player.setPlaybackEndCallback(endCb)
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      mockAudioElement.onended?.()
      expect(endCb).toHaveBeenCalled()
    })

    it('should fire error callback', async () => {
      const errCb = vi.fn()
      player.setPlaybackErrorCallback(errCb)
      const track = { id: 1, title: 'Test' } as any
      await player.loadTrack(track, 'http://localhost/stream/track/1')
      mockAudioElement.error = { code: 4, message: 'MEDIA_ERR' }
      mockAudioElement.onerror?.()
      expect(errCb).toHaveBeenCalled()
    })
  })

  describe('Stubs', () => {
    it('should return null for Web Audio stubs', () => {
      expect(player.getPipelineState()).toBeNull()
      expect(player.getEqualizer()).toBeNull()
      expect(player.getCompressor()).toBeNull()
      expect(player.getAnalyserNode()).toBeNull()
      expect(player.getAudioContext()).toBeNull()
      expect(player.getIsCrossfading()).toBe(false)
    })
  })
})
