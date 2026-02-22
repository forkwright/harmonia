import { describe, it, expect, beforeEach, vi } from 'vitest'

vi.mock('../services/syncService', () => ({
  syncService: {
    reportProgress: vi.fn().mockResolvedValue(undefined),
    startAutoSync: vi.fn().mockReturnValue(() => {}),
  },
}))

vi.mock('../services/sessionManager', () => ({
  sessionManager: {
    startSession: vi.fn().mockResolvedValue('mock-session-id'),
    endSession: vi.fn().mockResolvedValue(undefined),
  },
}))

import { usePlayerStore } from './playerStore'

describe('playerStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    usePlayerStore.setState({
      currentTrack: null,
      mediaItemId: null,
      queue: [],
      isPlaying: false,
      position: 0,
      duration: 0,
      volume: 1,
      playbackSpeed: 1,
      syncCleanup: null,
    })
  })

  describe('Initial State', () => {
    it('should have correct default state', () => {
      const state = usePlayerStore.getState()

      expect(state.currentTrack).toBe(null)
      expect(state.queue).toEqual([])
      expect(state.isPlaying).toBe(false)
      expect(state.position).toBe(0)
      expect(state.duration).toBe(0)
      expect(state.volume).toBe(1.0)
      expect(state.playbackSpeed).toBe(1.0)
    })
  })

  describe('Actions', () => {
    it('should set playing state', () => {
      const { setIsPlaying } = usePlayerStore.getState()

      setIsPlaying(true)
      expect(usePlayerStore.getState().isPlaying).toBe(true)

      setIsPlaying(false)
      expect(usePlayerStore.getState().isPlaying).toBe(false)
    })

    it('should update position', () => {
      const { setPosition } = usePlayerStore.getState()
      const newPosition = 45000 // 45 seconds in ms

      setPosition(newPosition)
      expect(usePlayerStore.getState().position).toBe(newPosition)
    })

    it('should update duration', () => {
      const { setDuration } = usePlayerStore.getState()
      const duration = 180000 // 3 minutes in ms

      setDuration(duration)
      expect(usePlayerStore.getState().duration).toBe(duration)
    })

    it('should update volume', () => {
      const { setVolume } = usePlayerStore.getState()

      setVolume(0.5)
      expect(usePlayerStore.getState().volume).toBe(0.5)

      setVolume(0.0)
      expect(usePlayerStore.getState().volume).toBe(0.0)
    })

    it('should update playback speed', () => {
      const { setPlaybackSpeed } = usePlayerStore.getState()

      setPlaybackSpeed(1.5)
      expect(usePlayerStore.getState().playbackSpeed).toBe(1.5)

      setPlaybackSpeed(0.5)
      expect(usePlayerStore.getState().playbackSpeed).toBe(0.5)
    })

    it('should clamp playback speed to valid range', () => {
      const { setPlaybackSpeed } = usePlayerStore.getState()

      setPlaybackSpeed(3.0) // Above max
      expect(usePlayerStore.getState().playbackSpeed).toBe(2.0)

      setPlaybackSpeed(0.1) // Below min
      expect(usePlayerStore.getState().playbackSpeed).toBe(0.5)
    })

    it('should clamp volume to valid range', () => {
      const { setVolume } = usePlayerStore.getState()

      setVolume(1.5) // Above max
      expect(usePlayerStore.getState().volume).toBe(1.0)

      setVolume(-0.5) // Below min
      expect(usePlayerStore.getState().volume).toBe(0.0)
    })
  })

  describe('Queue Management', () => {
    const mockTracks = [
      {
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
        channels: 2
      },
      {
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
        channels: 2
      },
      {
        id: 3,
        title: 'Track 3',
        artist: 'Artist 3',
        album: 'Album 3',
        duration: 220,
        fileSize: 7000000,
        format: 'FLAC',
        bitrate: 1411,
        sampleRate: 44100,
        bitDepth: 16,
        channels: 2
      },
    ]

    it('should add tracks to queue', () => {
      const { addToQueue } = usePlayerStore.getState()

      addToQueue(mockTracks[0])
      expect(usePlayerStore.getState().queue).toHaveLength(1)
      expect(usePlayerStore.getState().queue[0].id).toBe(1)

      addToQueue(mockTracks[1])
      expect(usePlayerStore.getState().queue).toHaveLength(2)
    })

    it('should remove track from queue by index', () => {
      const { addToQueue, removeFromQueue } = usePlayerStore.getState()

      mockTracks.forEach(track => addToQueue(track))
      expect(usePlayerStore.getState().queue).toHaveLength(3)

      removeFromQueue(1) // Remove second track (index 1)
      expect(usePlayerStore.getState().queue).toHaveLength(2)
      expect(usePlayerStore.getState().queue.find(t => t.id === 2)).toBeUndefined()
      expect(usePlayerStore.getState().queue[0].id).toBe(1)
      expect(usePlayerStore.getState().queue[1].id).toBe(3)
    })

    it('should clear queue', () => {
      const { addToQueue, clearQueue } = usePlayerStore.getState()

      mockTracks.forEach(track => addToQueue(track))
      expect(usePlayerStore.getState().queue).toHaveLength(3)

      clearQueue()
      expect(usePlayerStore.getState().queue).toHaveLength(0)
    })

    it('should set current track and reset position', () => {
      const { setCurrentTrack, setPosition } = usePlayerStore.getState()

      // Set some position first
      setPosition(50000)
      expect(usePlayerStore.getState().position).toBe(50000)

      // Set new track - should reset position to 0
      setCurrentTrack(mockTracks[0])

      const state = usePlayerStore.getState()
      expect(state.currentTrack).toEqual(mockTracks[0])
      expect(state.position).toBe(0)
    })

    it('should replace entire queue with setQueue', () => {
      const { addToQueue, setQueue } = usePlayerStore.getState()

      // Add some tracks first
      addToQueue(mockTracks[0])
      expect(usePlayerStore.getState().queue).toHaveLength(1)

      // Replace with new queue
      setQueue([mockTracks[1], mockTracks[2]])

      const queue = usePlayerStore.getState().queue
      expect(queue).toHaveLength(2)
      expect(queue[0].id).toBe(2)
      expect(queue[1].id).toBe(3)
    })

    it('should clear originalQueue when clearing queue', () => {
      usePlayerStore.setState({ queue: mockTracks, originalQueue: [...mockTracks] })
      usePlayerStore.getState().clearQueue()
      expect(usePlayerStore.getState().queue).toHaveLength(0)
      expect(usePlayerStore.getState().originalQueue).toHaveLength(0)
    })
  })

  describe('Repeat Mode', () => {
    const mockTracks = [
      { id: 1, title: 'Track 1', artist: 'A1', album: 'Al1', duration: 180, fileSize: 5e6, format: 'FLAC', bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2 },
      { id: 2, title: 'Track 2', artist: 'A2', album: 'Al2', duration: 200, fileSize: 6e6, format: 'FLAC', bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2 },
      { id: 3, title: 'Track 3', artist: 'A3', album: 'Al3', duration: 220, fileSize: 7e6, format: 'FLAC', bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2 },
    ]

    beforeEach(() => {
      localStorage.clear()
      usePlayerStore.setState({ repeatMode: 'off', originalQueue: [], queue: [] })
    })

    it('should default to off', () => {
      expect(usePlayerStore.getState().repeatMode).toBe('off')
    })

    it('should cycle through all four modes', () => {
      const { cycleRepeatMode } = usePlayerStore.getState()

      cycleRepeatMode()
      expect(usePlayerStore.getState().repeatMode).toBe('all')

      cycleRepeatMode()
      expect(usePlayerStore.getState().repeatMode).toBe('one')

      usePlayerStore.getState().cycleRepeatMode()
      expect(usePlayerStore.getState().repeatMode).toBe('shuffle-repeat')

      usePlayerStore.getState().cycleRepeatMode()
      expect(usePlayerStore.getState().repeatMode).toBe('off')
    })

    it('should persist repeat mode to localStorage', () => {
      usePlayerStore.getState().cycleRepeatMode()
      expect(JSON.parse(localStorage.getItem('akroasis_repeat_mode')!)).toBe('all')
    })

    it('should save originalQueue when entering shuffle-repeat', () => {
      usePlayerStore.setState({ queue: mockTracks })
      usePlayerStore.getState().setRepeatMode('shuffle-repeat')

      const state = usePlayerStore.getState()
      expect(state.originalQueue).toHaveLength(3)
      expect(state.originalQueue.map(t => t.id)).toEqual([1, 2, 3])
      expect(state.queue).toHaveLength(3)
    })

    it('should restore originalQueue when leaving shuffle-repeat', () => {
      usePlayerStore.setState({ queue: mockTracks })
      usePlayerStore.getState().setRepeatMode('shuffle-repeat')

      // Queue is now shuffled, originalQueue preserved
      usePlayerStore.getState().setRepeatMode('off')

      const state = usePlayerStore.getState()
      expect(state.originalQueue).toHaveLength(0)
      expect(state.queue.map(t => t.id)).toEqual([1, 2, 3])
    })

    it('should shuffle queue contents (not just copy)', () => {
      // Use enough tracks that shuffle is extremely unlikely to produce same order
      const manyTracks = Array.from({ length: 20 }, (_, i) => ({
        id: i, title: `T${i}`, artist: 'A', album: 'Al', duration: 180,
        fileSize: 5e6, format: 'FLAC' as const, bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2,
      }))
      usePlayerStore.setState({ queue: manyTracks })
      usePlayerStore.getState().setRepeatMode('shuffle-repeat')

      const shuffledIds = usePlayerStore.getState().queue.map(t => t.id)
      const originalIds = manyTracks.map(t => t.id)
      // Sorted should match (same elements), but order should differ
      expect([...shuffledIds].sort((a, b) => a - b)).toEqual(originalIds)
      expect(shuffledIds).not.toEqual(originalIds)
    })

    it('should set mode directly with setRepeatMode', () => {
      usePlayerStore.getState().setRepeatMode('one')
      expect(usePlayerStore.getState().repeatMode).toBe('one')
      expect(JSON.parse(localStorage.getItem('akroasis_repeat_mode')!)).toBe('one')
    })

    it('should handle cycling into shuffle-repeat via cycleRepeatMode', () => {
      usePlayerStore.setState({ queue: mockTracks, repeatMode: 'one' })
      usePlayerStore.getState().cycleRepeatMode() // one → shuffle-repeat
      expect(usePlayerStore.getState().repeatMode).toBe('shuffle-repeat')
      expect(usePlayerStore.getState().originalQueue).toHaveLength(3)
    })

    it('should handle cycling out of shuffle-repeat via cycleRepeatMode', () => {
      usePlayerStore.setState({ queue: mockTracks, repeatMode: 'one' })
      usePlayerStore.getState().cycleRepeatMode() // one → shuffle-repeat
      usePlayerStore.getState().cycleRepeatMode() // shuffle-repeat → off
      expect(usePlayerStore.getState().repeatMode).toBe('off')
      expect(usePlayerStore.getState().queue.map(t => t.id)).toEqual([1, 2, 3])
      expect(usePlayerStore.getState().originalQueue).toHaveLength(0)
    })
  })
})
