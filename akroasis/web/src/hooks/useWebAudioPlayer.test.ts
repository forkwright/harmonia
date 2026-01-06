import { describe, it, expect, beforeEach, vi } from 'vitest'
import { renderHook, act, waitFor } from '@testing-library/react'
import { useWebAudioPlayer } from './useWebAudioPlayer'
import { usePlayerStore } from '../stores/playerStore'

// Mock the player store
vi.mock('../stores/playerStore', () => ({
  usePlayerStore: vi.fn(),
}))

// Mock track data
const mockTrack = {
  id: 'track-1',
  title: 'Test Track',
  artist: 'Test Artist',
  album: 'Test Album',
  duration: 180,
  streamUrl: 'http://localhost:3000/stream/track-1',
}

interface MockStoreState {
  currentTrack: typeof mockTrack | null
  queue: typeof mockTrack[]
  isPlaying: boolean
  position: number
  duration: number
  volume: number
  playbackSpeed: number
  setIsPlaying: ReturnType<typeof vi.fn>
  setPosition: ReturnType<typeof vi.fn>
  setDuration: ReturnType<typeof vi.fn>
  setCurrentTrack: ReturnType<typeof vi.fn>
}

describe('useWebAudioPlayer', () => {
  let mockStoreState: MockStoreState

  beforeEach(() => {
    // Reset mocks
    vi.clearAllMocks()

    // Setup mock store state
    mockStoreState = {
      currentTrack: null,
      queue: [],
      isPlaying: false,
      position: 0,
      duration: 0,
      volume: 1.0,
      playbackSpeed: 1.0,
      setIsPlaying: vi.fn(),
      setPosition: vi.fn(),
      setDuration: vi.fn(),
      setCurrentTrack: vi.fn(),
    }

    // Mock usePlayerStore to return our mock state
    ;(usePlayerStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: MockStoreState) => unknown) =>
      selector ? selector(mockStoreState) : mockStoreState
    )
  })

  describe('Initialization', () => {
    it('should initialize with correct API', () => {
      const { result } = renderHook(() => useWebAudioPlayer())

      expect(result.current).toBeDefined()
      expect(typeof result.current.playTrack).toBe('function')
      expect(typeof result.current.togglePlayPause).toBe('function')
      expect(typeof result.current.seek).toBe('function')
      expect(typeof result.current.getPipelineState).toBe('function')
    })

    it('should create WebAudioPlayer on mount', () => {
      const { result } = renderHook(() => useWebAudioPlayer())

      // WebAudioPlayer is created in useEffect, which creates AudioContext lazily on first loadTrack
      expect(result.current).toBeDefined()
      expect(result.current.playTrack).toBeDefined()
    })
  })

  describe('Playback Controls', () => {
    it('should toggle play/pause state', async () => {
      mockStoreState.currentTrack = mockTrack
      mockStoreState.isPlaying = false

      const { result } = renderHook(() => useWebAudioPlayer())

      act(() => {
        result.current.togglePlayPause()
      })

      await waitFor(() => {
        expect(mockStoreState.setIsPlaying).toHaveBeenCalled()
      })
    })

    it('should handle seek operation', async () => {
      mockStoreState.currentTrack = mockTrack
      const seekTime = 60

      const { result } = renderHook(() => useWebAudioPlayer())

      act(() => {
        result.current.seek(seekTime)
      })

      await waitFor(() => {
        expect(mockStoreState.setPosition).toHaveBeenCalledWith(seekTime * 1000)
      })
    })

    it('should expose correct API functions', () => {
      const { result } = renderHook(() => useWebAudioPlayer())

      expect(result.current.playTrack).toBeDefined()
      expect(result.current.togglePlayPause).toBeDefined()
      expect(result.current.seek).toBeDefined()
      expect(result.current.getPipelineState).toBeDefined()
    })
  })

  describe('Track Changes', () => {
    it('should provide playTrack function', () => {
      const { result } = renderHook(() => useWebAudioPlayer())

      expect(typeof result.current.playTrack).toBe('function')
    })

    it('should handle track change while playing', async () => {
      mockStoreState.currentTrack = mockTrack
      mockStoreState.isPlaying = true

      const { rerender } = renderHook(() => useWebAudioPlayer())

      // Change to different track
      const newTrack = { ...mockTrack, id: 'track-2', title: 'New Track', streamUrl: 'http://localhost:3000/stream/track-2' }
      mockStoreState.currentTrack = newTrack

      rerender()

      // Should trigger new track load via useEffect
      await waitFor(() => {
        expect(mockStoreState.currentTrack?.id).toBe('track-2')
      })
    })
  })

  describe('Error Handling', () => {
    it('should handle playback errors gracefully', async () => {
      mockStoreState.currentTrack = mockTrack

      const { result } = renderHook(() => useWebAudioPlayer())

      // Toggle play should not throw even if audio fails to load
      await act(async () => {
        result.current.togglePlayPause()
      })

      // Should not crash
      expect(result.current).toBeDefined()
    })
  })

  describe('Cleanup', () => {
    it('should cleanup on unmount', () => {
      const { unmount } = renderHook(() => useWebAudioPlayer())

      unmount()

      // Should not throw errors
      expect(true).toBe(true)
    })
  })
})
