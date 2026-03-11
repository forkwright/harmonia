// Cross-media continue listening feed
import { create } from 'zustand'
import type { ContinueItem } from '../types'
import { apiClient } from '../api/client'

interface ContinueState {
  items: ContinueItem[]
  isLoading: boolean
  error: string | null

  fetchItems: (limit?: number) => Promise<void>
  clearItems: () => void
}

export const useContinueStore = create<ContinueState>((set) => ({
  items: [],
  isLoading: false,
  error: null,

  fetchItems: async (limit = 20) => {
    try {
      set({ isLoading: true, error: null })
      const items = await apiClient.getContinueListening(limit)
      set({ items, isLoading: false })
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : 'Failed to load continue items',
        isLoading: false,
      })
    }
  },

  clearItems: () => set({ items: [], error: null }),
}))
