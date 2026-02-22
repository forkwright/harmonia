// Podcast browsing and playback state
import { create } from 'zustand'
import type { PodcastShow, PodcastEpisode } from '../types'
import { apiClient } from '../api/client'
import { sessionManager } from '../services/sessionManager'

interface PodcastState {
  shows: PodcastShow[]
  selectedShow: PodcastShow | null
  episodes: PodcastEpisode[]
  currentEpisode: PodcastEpisode | null
  currentShow: PodcastShow | null
  isLoading: boolean
  error: string | null

  fetchShows: () => Promise<void>
  selectShow: (id: number) => Promise<void>
  clearSelection: () => void
  playEpisode: (episode: PodcastEpisode) => void
  clearPlayback: () => void
  subscribePodcast: (feedUrl: string) => Promise<void>
  unsubscribePodcast: (id: number) => Promise<void>
}

export const usePodcastStore = create<PodcastState>((set, get) => ({
  shows: [],
  selectedShow: null,
  episodes: [],
  currentEpisode: null,
  currentShow: null,
  isLoading: false,
  error: null,

  fetchShows: async () => {
    try {
      set({ isLoading: true, error: null })
      const result = await apiClient.getPodcasts()
      set({ shows: result.items, isLoading: false })
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to load podcasts', isLoading: false })
    }
  },

  selectShow: async (id: number) => {
    try {
      set({ isLoading: true, error: null })
      const [show, episodes] = await Promise.all([
        apiClient.getPodcast(id),
        apiClient.getPodcastEpisodes(id),
      ])
      episodes.sort((a, b) => {
        if (!a.publishDate) return 1
        if (!b.publishDate) return -1
        return new Date(b.publishDate).getTime() - new Date(a.publishDate).getTime()
      })
      set({ selectedShow: show, episodes, isLoading: false })
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to load podcast', isLoading: false })
    }
  },

  clearSelection: () => {
    set({ selectedShow: null, episodes: [] })
  },

  playEpisode: (episode) => {
    const prev = get().currentEpisode
    if (prev) void sessionManager.endSession(0)

    set({ currentEpisode: episode, currentShow: get().selectedShow })

    void sessionManager.startSession({
      mediaItemId: episode.id,
      mediaType: 'podcast',
      positionMs: 0,
      totalDurationMs: (episode.duration ?? 0) * 1000,
    })
  },

  clearPlayback: () => {
    const ep = get().currentEpisode
    if (ep) void sessionManager.endSession(0)
    set({ currentEpisode: null, currentShow: null })
  },

  subscribePodcast: async (feedUrl: string) => {
    try {
      set({ isLoading: true, error: null })
      await apiClient.addPodcast({
        title: '',
        feedUrl,
        monitored: true,
        monitorNewEpisodes: true,
        qualityProfileId: 1,
      } as Omit<PodcastShow, 'id' | 'added'>)
      const result = await apiClient.getPodcasts()
      set({ shows: result.items, isLoading: false })
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to add podcast', isLoading: false })
    }
  },

  unsubscribePodcast: async (id: number) => {
    try {
      set({ isLoading: true, error: null })
      await apiClient.deletePodcast(id)
      const { selectedShow, episodes } = get()
      set({
        shows: get().shows.filter((s) => s.id !== id),
        selectedShow: selectedShow?.id === id ? null : selectedShow,
        episodes: selectedShow?.id === id ? [] : episodes,
        isLoading: false,
      })
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to remove podcast', isLoading: false })
    }
  },
}))
