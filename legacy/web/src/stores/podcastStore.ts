// Podcast browsing and playback state
import { create } from 'zustand'
import type { PodcastShow, PodcastEpisode } from '../types'
import { logError } from '../utils/errorLogger'
import { apiClient } from '../api/client'
import { sessionManager } from '../services/sessionManager'

export type EpisodeFilter = 'all' | 'unplayed' | 'played'

interface PlayedRecord {
  played: boolean
  completedAt?: string
}

const LS_PLAYED = 'akroasis_podcast_played'
const LS_AUTO_MARK = 'akroasis_podcast_auto_mark_played'

function loadPlayed(): Record<number, PlayedRecord> {
  try {
    const raw = localStorage.getItem(LS_PLAYED)
    return raw ? JSON.parse(raw) : {}
  } catch { return {} }
}

function savePlayed(data: Record<number, PlayedRecord>) {
  localStorage.setItem(LS_PLAYED, JSON.stringify(data))
}

function loadAutoMark(): boolean {
  return localStorage.getItem(LS_AUTO_MARK) === 'true'
}

interface PodcastState {
  shows: PodcastShow[]
  selectedShow: PodcastShow | null
  episodes: PodcastEpisode[]
  currentEpisode: PodcastEpisode | null
  currentShow: PodcastShow | null
  isLoading: boolean
  error: string | null
  playedEpisodes: Record<number, PlayedRecord>
  episodeFilter: EpisodeFilter
  autoMarkPlayed: boolean

  fetchShows: () => Promise<void>
  selectShow: (id: number) => Promise<void>
  clearSelection: () => void
  playEpisode: (episode: PodcastEpisode) => void
  clearPlayback: () => void
  subscribePodcast: (feedUrl: string) => Promise<void>
  unsubscribePodcast: (id: number) => Promise<void>
  markPlayed: (episodeId: number) => void
  markUnplayed: (episodeId: number) => void
  togglePlayed: (episodeId: number) => void
  setEpisodeFilter: (filter: EpisodeFilter) => void
  setAutoMarkPlayed: (enabled: boolean) => void
}

export const usePodcastStore = create<PodcastState>((set, get) => ({
  shows: [],
  selectedShow: null,
  episodes: [],
  currentEpisode: null,
  currentShow: null,
  isLoading: false,
  error: null,
  playedEpisodes: loadPlayed(),
  episodeFilter: 'all' as EpisodeFilter,
  autoMarkPlayed: loadAutoMark(),

  fetchShows: async () => {
    try {
      set({ isLoading: true, error: null })
      const result = await apiClient.getPodcasts()
      set({ shows: result.items, isLoading: false })
    } catch (err) {
      logError('podcast', 'Failed to load podcasts', err)
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
      logError('podcast', 'Failed to load podcast', err)
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
    if (ep) {
      void sessionManager.endSession(0)
      if (get().autoMarkPlayed) {
        const played = { ...get().playedEpisodes, [ep.id]: { played: true, completedAt: new Date().toISOString() } }
        set({ playedEpisodes: played })
        savePlayed(played)
      }
    }
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

  markPlayed: (episodeId: number) => {
    const played = { ...get().playedEpisodes, [episodeId]: { played: true, completedAt: new Date().toISOString() } }
    set({ playedEpisodes: played })
    savePlayed(played)
  },

  markUnplayed: (episodeId: number) => {
    const played = { ...get().playedEpisodes }
    delete played[episodeId]
    set({ playedEpisodes: played })
    savePlayed(played)
  },

  togglePlayed: (episodeId: number) => {
    if (get().playedEpisodes[episodeId]?.played) {
      get().markUnplayed(episodeId)
    } else {
      get().markPlayed(episodeId)
    }
  },

  setEpisodeFilter: (filter: EpisodeFilter) => {
    set({ episodeFilter: filter })
  },

  setAutoMarkPlayed: (enabled: boolean) => {
    set({ autoMarkPlayed: enabled })
    localStorage.setItem(LS_AUTO_MARK, String(enabled))
  },
}))
