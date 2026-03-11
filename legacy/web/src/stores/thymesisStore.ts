// Favorites (Thymesis) — track-only, backend-persisted
import { create } from 'zustand'
import { apiClient } from '../api/client'
import type { Track, PagedResult } from '../types'

interface ThymesisState {
  favoriteIds: Set<number>
  loading: boolean

  isFavorite: (trackId: number) => boolean
  toggleFavorite: (trackId: number) => Promise<void>
  loadFavorites: () => Promise<void>
  getFavoriteTracks: (page?: number, pageSize?: number) => Promise<PagedResult<Track>>
}

export const useThymesisStore = create<ThymesisState>((set, get) => ({
  favoriteIds: new Set(),
  loading: false,

  isFavorite: (trackId) => get().favoriteIds.has(trackId),

  toggleFavorite: async (trackId) => {
    const { favoriteIds } = get()
    const wasFavorite = favoriteIds.has(trackId)

    // Optimistic update
    const next = new Set(favoriteIds)
    if (wasFavorite) {
      next.delete(trackId)
    } else {
      next.add(trackId)
    }
    set({ favoriteIds: next })

    try {
      if (wasFavorite) {
        await apiClient.removeFavorite(trackId)
      } else {
        await apiClient.addFavorite(trackId)
      }
    } catch {
      // Revert on failure
      const reverted = new Set(get().favoriteIds)
      if (wasFavorite) {
        reverted.add(trackId)
      } else {
        reverted.delete(trackId)
      }
      set({ favoriteIds: reverted })
    }
  },

  loadFavorites: async () => {
    set({ loading: true })
    try {
      const ids = await apiClient.getFavoriteIds()
      set({ favoriteIds: new Set(ids) })
    } catch {
      // Non-critical — favorites will load on retry
    } finally {
      set({ loading: false })
    }
  },

  getFavoriteTracks: async (page = 1, pageSize = 50) => {
    return apiClient.getFavorites(page, pageSize)
  },
}))
