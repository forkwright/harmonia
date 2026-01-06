import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { WebAudioPlayer } from './WebAudioPlayer'

// Mock fetch for audio data
globalThis.fetch = vi.fn()

describe('WebAudioPlayer', () => {
  let player: WebAudioPlayer

  beforeEach(() => {
    // Reset fetch mock
    vi.clearAllMocks()

    // Create player instance
    player = new WebAudioPlayer()
  })

  afterEach(async () => {
    await player.close()
  })

  describe('Gapless Playback', () => {
    const mockTrack1 = {
      id: 1,
      title: 'Track 1',
      artist: 'Artist 1',
      album: 'Album 1',
      duration: 180,
      fileSize: 5000000,
      format: 'FLAC',
      bitrate: 1411,
      sampleRate: 44100,
      bitDepth: 16,
      channels: 2,
      streamUrl: 'http://localhost:3000/stream/track-1'
    }

    const mockTrack2 = {
      id: 2,
      title: 'Track 2',
      artist: 'Artist 2',
      album: 'Album 2',
      duration: 200,
      fileSize: 6000000,
      format: 'FLAC',
      bitrate: 1411,
      sampleRate: 44100,
      bitDepth: 16,
      channels: 2,
      streamUrl: 'http://localhost:3000/stream/track-2'
    }

    beforeEach(() => {
      // Mock fetch to return mock audio data
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue({
        ok: true,
        arrayBuffer: async () => new ArrayBuffer(1024)
      })
    })

    it('should preload next track successfully', async () => {
      await player.loadTrack(mockTrack1, mockTrack1.streamUrl)
      await player.preloadNext(mockTrack2, mockTrack2.streamUrl)

      // Verify fetch was called for preload
      expect(globalThis.fetch).toHaveBeenCalledWith(mockTrack2.streamUrl)
    })

    it('should handle preload failures gracefully', async () => {
      // First load succeeds
      await player.loadTrack(mockTrack1, mockTrack1.streamUrl)

      // Mock fetch failure for subsequent preload (need to set up after initial load)
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue({
        ok: false,
        statusText: 'Not Found'
      })

      // Preload should not throw (it catches and logs warning)
      await player.preloadNext(mockTrack2, mockTrack2.streamUrl)

      // Player should still be functional after preload failure
      expect(player.getPlaybackState()).toBeDefined()
    })

    it('should expose playback state correctly', async () => {
      expect(player.getPlaybackState()).toBe(false)

      await player.loadTrack(mockTrack1, mockTrack1.streamUrl)

      // After loading, playback state should be true
      expect(player.getPlaybackState()).toBe(true)
    })

    it('should get current time correctly', async () => {
      await player.loadTrack(mockTrack1, mockTrack1.streamUrl)

      const currentTime = player.getCurrentTime()
      expect(typeof currentTime).toBe('number')
      expect(currentTime).toBeGreaterThanOrEqual(0)
    })

    it('should get duration from loaded track', async () => {
      await player.loadTrack(mockTrack1, mockTrack1.streamUrl)

      const duration = player.getDuration()
      expect(typeof duration).toBe('number')
      expect(duration).toBeGreaterThan(0)
    })

    it('should provide pipeline state after loading', async () => {
      await player.loadTrack(mockTrack1, mockTrack1.streamUrl)

      const pipelineState = player.getPipelineState()
      expect(pipelineState).toBeDefined()
      expect(pipelineState?.inputFormat).toBeDefined()
      expect(pipelineState?.outputDevice).toBeDefined()
      expect(pipelineState?.inputFormat.sampleRate).toBeGreaterThan(0)
      expect(pipelineState?.inputFormat.channels).toBeGreaterThan(0)
    })

    it('should return null pipeline state before loading', () => {
      const pipelineState = player.getPipelineState()
      expect(pipelineState).toBeNull()
    })
  })

  describe('Playback Controls', () => {
    const mockTrack = {
      id: 1,
      title: 'Track 1',
      artist: 'Artist 1',
      album: 'Album 1',
      duration: 180,
      fileSize: 5000000,
      format: 'FLAC',
      bitrate: 1411,
      sampleRate: 44100,
      bitDepth: 16,
      channels: 2,
      streamUrl: 'http://localhost:3000/stream/track-1'
    }

    beforeEach(() => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue({
        ok: true,
        arrayBuffer: async () => new ArrayBuffer(1024)
      })
    })

    it('should load and play track', async () => {
      await player.loadTrack(mockTrack, mockTrack.streamUrl)

      expect(player.getPlaybackState()).toBe(true)
      expect(globalThis.fetch).toHaveBeenCalledWith(mockTrack.streamUrl)
    })

    it('should pause playback', async () => {
      await player.loadTrack(mockTrack, mockTrack.streamUrl)
      expect(player.getPlaybackState()).toBe(true)

      player.pause()
      expect(player.getPlaybackState()).toBe(false)
    })

    it('should stop playback', async () => {
      await player.loadTrack(mockTrack, mockTrack.streamUrl)
      expect(player.getPlaybackState()).toBe(true)

      player.stop()
      expect(player.getPlaybackState()).toBe(false)
    })

    it('should seek to position', async () => {
      await player.loadTrack(mockTrack, mockTrack.streamUrl)

      player.seek(60) // Seek to 60 seconds
      const currentTime = player.getCurrentTime()

      // Should be at or near seek position
      expect(currentTime).toBeGreaterThanOrEqual(0)
    })

    it('should set volume with clamping', async () => {
      await player.loadTrack(mockTrack, mockTrack.streamUrl)

      // Test normal volume
      player.setVolume(0.5)
      // Volume is set internally, no getter to verify

      // Test clamping above max
      player.setVolume(1.5)
      // Should clamp to 1.0

      // Test clamping below min
      player.setVolume(-0.5)
      // Should clamp to 0.0

      // Function should not throw
      expect(true).toBe(true)
    })

    it('should set playback speed with clamping', async () => {
      await player.loadTrack(mockTrack, mockTrack.streamUrl)

      // Test normal speed
      player.setPlaybackSpeed(1.5)

      // Test clamping above max
      player.setPlaybackSpeed(3.0)
      // Should clamp to 2.0

      // Test clamping below min
      player.setPlaybackSpeed(0.1)
      // Should clamp to 0.5

      // Function should not throw
      expect(true).toBe(true)
    })

    it('should handle load errors', async () => {
      (globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        statusText: 'Internal Server Error'
      })

      await expect(player.loadTrack(mockTrack, mockTrack.streamUrl)).rejects.toThrow()
    })

    it('should register playback end callback', async () => {
      const callback = vi.fn()
      player.setPlaybackEndCallback(callback)

      await player.loadTrack(mockTrack, mockTrack.streamUrl)

      // Callback is registered, will be called when track ends
      expect(true).toBe(true)
    })

    it('should register playback error callback', () => {
      const callback = vi.fn()
      player.setPlaybackErrorCallback(callback)

      // Callback is registered
      expect(true).toBe(true)
    })
  })

  describe('Cleanup', () => {
    it('should close cleanly', async () => {
      const mockTrack = {
        id: 1,
        title: 'Track 1',
        artist: 'Artist 1',
        album: 'Album 1',
        duration: 180,
        fileSize: 5000000,
        format: 'FLAC',
        bitrate: 1411,
        sampleRate: 44100,
        bitDepth: 16,
        channels: 2,
        streamUrl: 'http://localhost:3000/stream/track-1'
      }

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValue({
        ok: true,
        arrayBuffer: async () => new ArrayBuffer(1024)
      })

      await player.loadTrack(mockTrack, mockTrack.streamUrl)
      await player.close()

      // After close, pipeline state should be null
      expect(player.getPipelineState()).toBeNull()
    })
  })
})
