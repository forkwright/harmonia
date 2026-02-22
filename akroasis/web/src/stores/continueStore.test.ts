import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useContinueStore } from './continueStore'

vi.mock('../api/client', () => ({
  apiClient: {
    getContinueListening: vi.fn(),
  },
}))

import { apiClient } from '../api/client'

const mockGetContinue = apiClient.getContinueListening as ReturnType<typeof vi.fn>

const mockItems = [
  {
    mediaItemId: 1,
    title: 'Test Audiobook',
    mediaType: 'audiobook',
    positionMs: 120000,
    totalDurationMs: 600000,
    percentComplete: 20,
    lastPlayedAt: '2026-02-20T12:00:00Z',
    coverUrl: '/cover/1.jpg',
  },
  {
    mediaItemId: 2,
    title: 'Test Song',
    mediaType: 'music',
    positionMs: 60000,
    totalDurationMs: 240000,
    percentComplete: 25,
    lastPlayedAt: '2026-02-20T11:00:00Z',
    coverUrl: '/cover/2.jpg',
  },
]

describe('continueStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useContinueStore.setState({ items: [], isLoading: false, error: null })
  })

  it('has correct initial state', () => {
    const state = useContinueStore.getState()
    expect(state.items).toEqual([])
    expect(state.isLoading).toBe(false)
    expect(state.error).toBeNull()
  })

  describe('fetchItems', () => {
    it('loads continue items', async () => {
      mockGetContinue.mockResolvedValueOnce(mockItems)

      await useContinueStore.getState().fetchItems()

      const state = useContinueStore.getState()
      expect(state.items).toHaveLength(2)
      expect(state.items[0].title).toBe('Test Audiobook')
      expect(state.isLoading).toBe(false)
      expect(state.error).toBeNull()
    })

    it('passes limit parameter', async () => {
      mockGetContinue.mockResolvedValueOnce([])

      await useContinueStore.getState().fetchItems(5)

      expect(mockGetContinue).toHaveBeenCalledWith(5)
    })

    it('defaults limit to 20', async () => {
      mockGetContinue.mockResolvedValueOnce([])

      await useContinueStore.getState().fetchItems()

      expect(mockGetContinue).toHaveBeenCalledWith(20)
    })

    it('sets loading state', async () => {
      let resolvePromise: (value: unknown[]) => void
      mockGetContinue.mockReturnValueOnce(new Promise((resolve) => { resolvePromise = resolve }))

      const fetchPromise = useContinueStore.getState().fetchItems()
      expect(useContinueStore.getState().isLoading).toBe(true)

      resolvePromise!(mockItems)
      await fetchPromise
      expect(useContinueStore.getState().isLoading).toBe(false)
    })

    it('handles errors', async () => {
      mockGetContinue.mockRejectedValueOnce(new Error('Network error'))

      await useContinueStore.getState().fetchItems()

      const state = useContinueStore.getState()
      expect(state.error).toBe('Network error')
      expect(state.isLoading).toBe(false)
      expect(state.items).toEqual([])
    })

    it('clears previous error on new fetch', async () => {
      useContinueStore.setState({ error: 'old error' })
      mockGetContinue.mockResolvedValueOnce(mockItems)

      await useContinueStore.getState().fetchItems()

      expect(useContinueStore.getState().error).toBeNull()
    })
  })

  describe('clearItems', () => {
    it('resets items and error', () => {
      useContinueStore.setState({ items: mockItems, error: 'some error' })

      useContinueStore.getState().clearItems()

      const state = useContinueStore.getState()
      expect(state.items).toEqual([])
      expect(state.error).toBeNull()
    })
  })
})
