// Thymesis (favorites) store tests
import { describe, it, expect, beforeEach, vi } from 'vitest'

const mockAddFavorite = vi.fn().mockResolvedValue(undefined)
const mockRemoveFavorite = vi.fn().mockResolvedValue(undefined)
const mockGetFavoriteIds = vi.fn().mockResolvedValue([1, 3])
const mockGetFavorites = vi.fn().mockResolvedValue({ items: [], page: 1, pageSize: 50, totalCount: 0 })

vi.mock('../api/client', () => ({
  apiClient: {
    addFavorite: (...args: unknown[]) => mockAddFavorite(...args),
    removeFavorite: (...args: unknown[]) => mockRemoveFavorite(...args),
    getFavoriteIds: (...args: unknown[]) => mockGetFavoriteIds(...args),
    getFavorites: (...args: unknown[]) => mockGetFavorites(...args),
  },
}))

import { useThymesisStore } from './thymesisStore'

describe('thymesisStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useThymesisStore.setState({ favoriteIds: new Set(), loading: false })
  })

  describe('isFavorite', () => {
    it('returns false for non-favorited track', () => {
      expect(useThymesisStore.getState().isFavorite(99)).toBe(false)
    })

    it('returns true for favorited track', () => {
      useThymesisStore.setState({ favoriteIds: new Set([1, 3]) })
      expect(useThymesisStore.getState().isFavorite(1)).toBe(true)
      expect(useThymesisStore.getState().isFavorite(3)).toBe(true)
    })
  })

  describe('toggleFavorite', () => {
    it('adds a favorite optimistically', async () => {
      await useThymesisStore.getState().toggleFavorite(5)

      expect(useThymesisStore.getState().favoriteIds.has(5)).toBe(true)
      expect(mockAddFavorite).toHaveBeenCalledWith(5)
    })

    it('removes a favorite optimistically', async () => {
      useThymesisStore.setState({ favoriteIds: new Set([5]) })
      await useThymesisStore.getState().toggleFavorite(5)

      expect(useThymesisStore.getState().favoriteIds.has(5)).toBe(false)
      expect(mockRemoveFavorite).toHaveBeenCalledWith(5)
    })

    it('reverts on API failure when adding', async () => {
      mockAddFavorite.mockRejectedValueOnce(new Error('Network error'))
      await useThymesisStore.getState().toggleFavorite(5)

      expect(useThymesisStore.getState().favoriteIds.has(5)).toBe(false)
    })

    it('reverts on API failure when removing', async () => {
      useThymesisStore.setState({ favoriteIds: new Set([5]) })
      mockRemoveFavorite.mockRejectedValueOnce(new Error('Network error'))
      await useThymesisStore.getState().toggleFavorite(5)

      expect(useThymesisStore.getState().favoriteIds.has(5)).toBe(true)
    })
  })

  describe('loadFavorites', () => {
    it('loads favorite ids from API', async () => {
      await useThymesisStore.getState().loadFavorites()

      expect(useThymesisStore.getState().favoriteIds).toEqual(new Set([1, 3]))
      expect(useThymesisStore.getState().loading).toBe(false)
    })

    it('sets loading state during load', async () => {
      let resolvePromise: (value: number[]) => void
      mockGetFavoriteIds.mockReturnValueOnce(new Promise<number[]>((resolve) => {
        resolvePromise = resolve
      }))

      const loadPromise = useThymesisStore.getState().loadFavorites()
      expect(useThymesisStore.getState().loading).toBe(true)

      resolvePromise!([1])
      await loadPromise
      expect(useThymesisStore.getState().loading).toBe(false)
    })

    it('handles API error gracefully', async () => {
      mockGetFavoriteIds.mockRejectedValueOnce(new Error('fail'))
      await useThymesisStore.getState().loadFavorites()

      expect(useThymesisStore.getState().favoriteIds).toEqual(new Set())
      expect(useThymesisStore.getState().loading).toBe(false)
    })
  })

  describe('getFavoriteTracks', () => {
    it('delegates to apiClient.getFavorites', async () => {
      await useThymesisStore.getState().getFavoriteTracks(2, 25)
      expect(mockGetFavorites).toHaveBeenCalledWith(2, 25)
    })
  })
})
