import { describe, it, expect, beforeEach, vi } from 'vitest'
import { usePodcastStore } from './podcastStore'
import { apiClient } from '../api/client'
import type { PodcastShow, PodcastEpisode } from '../types'

vi.mock('../api/client', () => ({
  apiClient: {
    getPodcasts: vi.fn(),
    getPodcast: vi.fn(),
    getPodcastEpisodes: vi.fn(),
    addPodcast: vi.fn(),
    deletePodcast: vi.fn(),
  },
}))

vi.mock('../services/sessionManager', () => ({
  sessionManager: {
    startSession: vi.fn().mockResolvedValue('mock-session-id'),
    endSession: vi.fn().mockResolvedValue(undefined),
  },
}))

const mockShow1: PodcastShow = {
  id: 301,
  title: 'Lex Fridman Podcast',
  author: 'Lex Fridman',
  feedUrl: 'https://lexfridman.com/feed/podcast/',
  episodeCount: 412,
  monitored: true,
  monitorNewEpisodes: true,
  qualityProfileId: 1,
  added: '2026-01-10T00:00:00Z',
}

const mockShow2: PodcastShow = {
  id: 302,
  title: 'Huberman Lab',
  author: 'Andrew Huberman',
  feedUrl: 'https://hubermanlab.com/feed/',
  episodeCount: 187,
  monitored: true,
  monitorNewEpisodes: true,
  qualityProfileId: 1,
  added: '2026-01-12T00:00:00Z',
}

const mockEpisodes: PodcastEpisode[] = [
  {
    id: 4001,
    podcastShowId: 301,
    title: 'Sam Altman: OpenAI and GPT-5',
    episodeNumber: 367,
    publishDate: '2026-02-18T00:00:00Z',
    duration: 10440,
    enclosureUrl: 'https://example.com/ep367.mp3',
    enclosureType: 'audio/mpeg',
    explicit: false,
    monitored: true,
    added: '2026-02-18T01:00:00Z',
  },
  {
    id: 4002,
    podcastShowId: 301,
    title: 'Elon Musk: War, AI, Aliens',
    episodeNumber: 400,
    publishDate: '2026-01-10T00:00:00Z',
    duration: 14100,
    enclosureUrl: 'https://example.com/ep400.mp3',
    enclosureType: 'audio/mpeg',
    explicit: false,
    monitored: true,
    added: '2026-01-10T01:00:00Z',
  },
]

const pagedShows = {
  items: [mockShow1, mockShow2],
  page: 1,
  pageSize: 50,
  totalCount: 2,
}

function resetStore() {
  usePodcastStore.setState({
    shows: [],
    selectedShow: null,
    episodes: [],
    currentEpisode: null,
    currentShow: null,
    isLoading: false,
    error: null,
    playedEpisodes: {},
    episodeFilter: 'all',
    autoMarkPlayed: false,
  })
}

describe('podcastStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  describe('fetchShows', () => {
    it('loads shows and clears loading state on success', async () => {
      vi.mocked(apiClient.getPodcasts).mockResolvedValueOnce(pagedShows)

      await usePodcastStore.getState().fetchShows()

      const state = usePodcastStore.getState()
      expect(state.shows).toEqual([mockShow1, mockShow2])
      expect(state.isLoading).toBe(false)
      expect(state.error).toBeNull()
    })

    it('sets isLoading to true while fetching', async () => {
      let resolvePromise!: (value: typeof pagedShows) => void
      vi.mocked(apiClient.getPodcasts).mockReturnValueOnce(
        new Promise((resolve) => { resolvePromise = resolve })
      )

      const fetchPromise = usePodcastStore.getState().fetchShows()
      expect(usePodcastStore.getState().isLoading).toBe(true)

      resolvePromise(pagedShows)
      await fetchPromise

      expect(usePodcastStore.getState().isLoading).toBe(false)
    })

    it('sets error and clears loading on failure', async () => {
      vi.mocked(apiClient.getPodcasts).mockRejectedValueOnce(new Error('Network timeout'))

      await usePodcastStore.getState().fetchShows()

      const state = usePodcastStore.getState()
      expect(state.error).toBe('Network timeout')
      expect(state.isLoading).toBe(false)
      expect(state.shows).toEqual([])
    })

    it('sets generic error message when error has no message', async () => {
      vi.mocked(apiClient.getPodcasts).mockRejectedValueOnce('unexpected')

      await usePodcastStore.getState().fetchShows()

      expect(usePodcastStore.getState().error).toBe('Failed to load podcasts')
    })

    it('calls getPodcasts with default page and pageSize', async () => {
      vi.mocked(apiClient.getPodcasts).mockResolvedValueOnce(pagedShows)

      await usePodcastStore.getState().fetchShows()

      expect(apiClient.getPodcasts).toHaveBeenCalledOnce()
    })
  })

  describe('selectShow', () => {
    it('sets selectedShow and fetches episodes for the given id', async () => {
      vi.mocked(apiClient.getPodcast).mockResolvedValueOnce(mockShow1)
      vi.mocked(apiClient.getPodcastEpisodes).mockResolvedValueOnce([...mockEpisodes])

      await usePodcastStore.getState().selectShow(301)

      const state = usePodcastStore.getState()
      expect(state.selectedShow).toEqual(mockShow1)
      expect(state.episodes).toHaveLength(2)
      expect(state.isLoading).toBe(false)
      expect(state.error).toBeNull()
    })

    it('sorts episodes by publishDate descending', async () => {
      const unsorted = [
        { ...mockEpisodes[1] },
        { ...mockEpisodes[0] },
      ]
      vi.mocked(apiClient.getPodcast).mockResolvedValueOnce(mockShow1)
      vi.mocked(apiClient.getPodcastEpisodes).mockResolvedValueOnce(unsorted)

      await usePodcastStore.getState().selectShow(301)

      const eps = usePodcastStore.getState().episodes
      expect(eps[0].id).toBe(4001)
      expect(eps[1].id).toBe(4002)
    })

    it('puts episodes without publishDate at the end', async () => {
      const withMissing: PodcastEpisode[] = [
        { ...mockEpisodes[0], publishDate: undefined },
        { ...mockEpisodes[1] },
      ]
      vi.mocked(apiClient.getPodcast).mockResolvedValueOnce(mockShow1)
      vi.mocked(apiClient.getPodcastEpisodes).mockResolvedValueOnce(withMissing)

      await usePodcastStore.getState().selectShow(301)

      const eps = usePodcastStore.getState().episodes
      expect(eps[0].id).toBe(4002)
      expect(eps[1].id).toBe(4001)
    })

    it('fetches show and episodes in parallel', async () => {
      vi.mocked(apiClient.getPodcast).mockResolvedValueOnce(mockShow2)
      vi.mocked(apiClient.getPodcastEpisodes).mockResolvedValueOnce([])

      await usePodcastStore.getState().selectShow(302)

      expect(apiClient.getPodcast).toHaveBeenCalledWith(302)
      expect(apiClient.getPodcastEpisodes).toHaveBeenCalledWith(302)
    })

    it('sets isLoading to true while fetching', async () => {
      let resolveShow!: (v: PodcastShow) => void
      let resolveEpisodes!: (v: PodcastEpisode[]) => void

      vi.mocked(apiClient.getPodcast).mockReturnValueOnce(
        new Promise((r) => { resolveShow = r })
      )
      vi.mocked(apiClient.getPodcastEpisodes).mockReturnValueOnce(
        new Promise((r) => { resolveEpisodes = r })
      )

      const p = usePodcastStore.getState().selectShow(301)
      expect(usePodcastStore.getState().isLoading).toBe(true)

      resolveShow(mockShow1)
      resolveEpisodes(mockEpisodes)
      await p

      expect(usePodcastStore.getState().isLoading).toBe(false)
    })

    it('sets error on failure and clears loading', async () => {
      vi.mocked(apiClient.getPodcast).mockRejectedValueOnce(new Error('Not found'))
      vi.mocked(apiClient.getPodcastEpisodes).mockRejectedValueOnce(new Error('Not found'))

      await usePodcastStore.getState().selectShow(999)

      const state = usePodcastStore.getState()
      expect(state.error).toBe('Not found')
      expect(state.isLoading).toBe(false)
      expect(state.selectedShow).toBeNull()
    })

    it('sets empty episodes when show has none', async () => {
      vi.mocked(apiClient.getPodcast).mockResolvedValueOnce(mockShow2)
      vi.mocked(apiClient.getPodcastEpisodes).mockResolvedValueOnce([])

      await usePodcastStore.getState().selectShow(302)

      expect(usePodcastStore.getState().episodes).toEqual([])
    })
  })

  describe('clearSelection', () => {
    it('clears selectedShow and episodes', () => {
      usePodcastStore.setState({ selectedShow: mockShow1, episodes: mockEpisodes })

      usePodcastStore.getState().clearSelection()

      const state = usePodcastStore.getState()
      expect(state.selectedShow).toBeNull()
      expect(state.episodes).toEqual([])
    })

    it('does not affect shows list', () => {
      usePodcastStore.setState({ shows: [mockShow1, mockShow2], selectedShow: mockShow1 })

      usePodcastStore.getState().clearSelection()

      expect(usePodcastStore.getState().shows).toEqual([mockShow1, mockShow2])
    })

    it('does not affect error or loading state', () => {
      usePodcastStore.setState({
        selectedShow: mockShow1,
        episodes: mockEpisodes,
        error: 'some error',
        isLoading: false,
      })

      usePodcastStore.getState().clearSelection()

      expect(usePodcastStore.getState().error).toBe('some error')
      expect(usePodcastStore.getState().isLoading).toBe(false)
    })
  })

  describe('playEpisode', () => {
    it('sets currentEpisode and currentShow from selectedShow', () => {
      usePodcastStore.setState({ selectedShow: mockShow1 })

      usePodcastStore.getState().playEpisode(mockEpisodes[0])

      const state = usePodcastStore.getState()
      expect(state.currentEpisode).toEqual(mockEpisodes[0])
      expect(state.currentShow).toEqual(mockShow1)
    })

    it('sets currentShow to null when no show is selected', () => {
      usePodcastStore.getState().playEpisode(mockEpisodes[0])

      expect(usePodcastStore.getState().currentShow).toBeNull()
    })
  })

  describe('clearPlayback', () => {
    it('clears currentEpisode and currentShow', () => {
      usePodcastStore.setState({
        currentEpisode: mockEpisodes[0],
        currentShow: mockShow1,
      })

      usePodcastStore.getState().clearPlayback()

      const state = usePodcastStore.getState()
      expect(state.currentEpisode).toBeNull()
      expect(state.currentShow).toBeNull()
    })
  })

  describe('subscribePodcast', () => {
    it('calls addPodcast and re-fetches shows', async () => {
      vi.mocked(apiClient.addPodcast).mockResolvedValueOnce(mockShow1)
      vi.mocked(apiClient.getPodcasts).mockResolvedValueOnce(pagedShows)

      await usePodcastStore.getState().subscribePodcast('https://example.com/feed.xml')

      expect(apiClient.addPodcast).toHaveBeenCalledWith(expect.objectContaining({
        feedUrl: 'https://example.com/feed.xml',
        monitored: true,
      }))
      expect(usePodcastStore.getState().shows).toEqual([mockShow1, mockShow2])
      expect(usePodcastStore.getState().isLoading).toBe(false)
    })

    it('sets error on failure', async () => {
      vi.mocked(apiClient.addPodcast).mockRejectedValueOnce(new Error('Invalid feed'))

      await usePodcastStore.getState().subscribePodcast('bad-url')

      expect(usePodcastStore.getState().error).toBe('Invalid feed')
      expect(usePodcastStore.getState().isLoading).toBe(false)
    })
  })

  describe('unsubscribePodcast', () => {
    it('removes show from list after deletion', async () => {
      usePodcastStore.setState({ shows: [mockShow1, mockShow2] })
      vi.mocked(apiClient.deletePodcast).mockResolvedValueOnce(undefined)

      await usePodcastStore.getState().unsubscribePodcast(301)

      expect(apiClient.deletePodcast).toHaveBeenCalledWith(301)
      expect(usePodcastStore.getState().shows).toEqual([mockShow2])
      expect(usePodcastStore.getState().isLoading).toBe(false)
    })

    it('clears selection if unsubscribed show was selected', async () => {
      usePodcastStore.setState({
        shows: [mockShow1, mockShow2],
        selectedShow: mockShow1,
        episodes: mockEpisodes,
      })
      vi.mocked(apiClient.deletePodcast).mockResolvedValueOnce(undefined)

      await usePodcastStore.getState().unsubscribePodcast(301)

      expect(usePodcastStore.getState().selectedShow).toBeNull()
      expect(usePodcastStore.getState().episodes).toEqual([])
    })

    it('preserves selection if different show was unsubscribed', async () => {
      usePodcastStore.setState({
        shows: [mockShow1, mockShow2],
        selectedShow: mockShow1,
        episodes: mockEpisodes,
      })
      vi.mocked(apiClient.deletePodcast).mockResolvedValueOnce(undefined)

      await usePodcastStore.getState().unsubscribePodcast(302)

      expect(usePodcastStore.getState().selectedShow).toEqual(mockShow1)
      expect(usePodcastStore.getState().episodes).toEqual(mockEpisodes)
    })

    it('sets error on failure', async () => {
      usePodcastStore.setState({ shows: [mockShow1] })
      vi.mocked(apiClient.deletePodcast).mockRejectedValueOnce(new Error('Forbidden'))

      await usePodcastStore.getState().unsubscribePodcast(301)

      expect(usePodcastStore.getState().error).toBe('Forbidden')
      expect(usePodcastStore.getState().shows).toEqual([mockShow1])
    })
  })

  describe('initial state', () => {
    it('has empty shows, no selected show, and no episodes by default', () => {
      const state = usePodcastStore.getState()
      expect(state.shows).toEqual([])
      expect(state.selectedShow).toBeNull()
      expect(state.episodes).toEqual([])
      expect(state.currentEpisode).toBeNull()
      expect(state.currentShow).toBeNull()
      expect(state.isLoading).toBe(false)
      expect(state.error).toBeNull()
    })
  })

  describe('markPlayed / markUnplayed', () => {
    it('marks episode as played', () => {
      usePodcastStore.getState().markPlayed(4001)
      const state = usePodcastStore.getState()
      expect(state.playedEpisodes[4001]?.played).toBe(true)
      expect(state.playedEpisodes[4001]?.completedAt).toBeDefined()
    })

    it('persists played state to localStorage', () => {
      usePodcastStore.getState().markPlayed(4001)
      const stored = localStorage.getItem('akroasis_podcast_played')
      expect(stored).toBeTruthy()
      expect(JSON.parse(stored!)[4001].played).toBe(true)
    })

    it('marks episode as unplayed by removing the record', () => {
      usePodcastStore.getState().markPlayed(4001)
      usePodcastStore.getState().markUnplayed(4001)
      expect(usePodcastStore.getState().playedEpisodes[4001]).toBeUndefined()
    })
  })

  describe('togglePlayed', () => {
    it('toggles unplayed to played', () => {
      usePodcastStore.getState().togglePlayed(4001)
      expect(usePodcastStore.getState().playedEpisodes[4001]?.played).toBe(true)
    })

    it('toggles played to unplayed', () => {
      usePodcastStore.getState().markPlayed(4001)
      usePodcastStore.getState().togglePlayed(4001)
      expect(usePodcastStore.getState().playedEpisodes[4001]).toBeUndefined()
    })
  })

  describe('episodeFilter', () => {
    it('defaults to all', () => {
      expect(usePodcastStore.getState().episodeFilter).toBe('all')
    })

    it('sets filter to unplayed', () => {
      usePodcastStore.getState().setEpisodeFilter('unplayed')
      expect(usePodcastStore.getState().episodeFilter).toBe('unplayed')
    })

    it('sets filter to played', () => {
      usePodcastStore.getState().setEpisodeFilter('played')
      expect(usePodcastStore.getState().episodeFilter).toBe('played')
    })
  })

  describe('autoMarkPlayed', () => {
    it('defaults to false', () => {
      expect(usePodcastStore.getState().autoMarkPlayed).toBe(false)
    })

    it('persists to localStorage', () => {
      usePodcastStore.getState().setAutoMarkPlayed(true)
      expect(localStorage.getItem('akroasis_podcast_auto_mark_played')).toBe('true')
    })

    it('auto-marks played on clearPlayback when enabled', () => {
      usePodcastStore.setState({ currentEpisode: mockEpisodes[0], autoMarkPlayed: true, playedEpisodes: {} })
      usePodcastStore.getState().clearPlayback()
      expect(usePodcastStore.getState().playedEpisodes[4001]?.played).toBe(true)
    })

    it('does not auto-mark when disabled', () => {
      usePodcastStore.setState({ currentEpisode: mockEpisodes[0], autoMarkPlayed: false, playedEpisodes: {} })
      usePodcastStore.getState().clearPlayback()
      expect(usePodcastStore.getState().playedEpisodes[4001]).toBeUndefined()
    })
  })
})
