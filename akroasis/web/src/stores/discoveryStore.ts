// Discovery page state — sessions, history, tracks for intelligence
import { create } from 'zustand'
import type { PlaybackSession, HistoryEntry, Track } from '../types'
import { apiClient } from '../api/client'

interface DiscoveryState {
  sessions: PlaybackSession[]
  recentHistory: HistoryEntry[]
  tracks: Track[]
  isLoading: boolean
  error: string | null

  fetchSessions: () => Promise<void>
  fetchHistory: () => Promise<void>
  fetchTracks: () => Promise<void>
  fetchAll: () => Promise<void>
}

export const useDiscoveryStore = create<DiscoveryState>((set) => ({
  sessions: [],
  recentHistory: [],
  tracks: [],
  isLoading: false,
  error: null,

  fetchSessions: async () => {
    try {
      set({ isLoading: true, error: null })
      const sessions = await apiClient.getSessions()
      set({ sessions, isLoading: false })
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : 'Failed to load sessions',
        isLoading: false,
      })
    }
  },

  fetchHistory: async () => {
    try {
      set({ isLoading: true, error: null })
      const result = await apiClient.getHistory(1, 20)
      set({ recentHistory: result.records, isLoading: false })
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : 'Failed to load history',
        isLoading: false,
      })
    }
  },

  fetchTracks: async () => {
    try {
      set({ isLoading: true, error: null })
      const tracks = await apiClient.getTracks()
      set({ tracks, isLoading: false })
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : 'Failed to load tracks',
        isLoading: false,
      })
    }
  },

  fetchAll: async () => {
    set({ isLoading: true, error: null })
    try {
      const [sessions, historyResult, tracks] = await Promise.all([
        apiClient.getSessions(),
        apiClient.getHistory(1, 20),
        apiClient.getTracks(),
      ])
      set({
        sessions,
        recentHistory: historyResult.records,
        tracks,
        isLoading: false,
      })
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : 'Failed to load discovery data',
        isLoading: false,
      })
    }
  },
}))
