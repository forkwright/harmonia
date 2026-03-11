import { describe, it, expect, beforeEach, vi } from 'vitest'
import { useSearchStore } from './searchStore'
import { apiClient } from '../api/client'
import { useAudiobookStore } from './audiobookStore'
import { usePodcastStore } from './podcastStore'

vi.mock('../api/client', () => ({
  apiClient: {
    search: vi.fn(),
  },
}))

vi.mock('./audiobookStore', () => ({
  useAudiobookStore: {
    getState: vi.fn().mockReturnValue({ audiobooks: [], authors: [] }),
  },
}))

vi.mock('./podcastStore', () => ({
  usePodcastStore: {
    getState: vi.fn().mockReturnValue({ shows: [] }),
  },
}))

function resetStore() {
  useSearchStore.setState({
    query: '',
    results: [],
    isSearching: false,
    isOpen: false,
    selectedIndex: -1,
  })
}

describe('searchStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  describe('initial state', () => {
    it('has empty query and no results', () => {
      const state = useSearchStore.getState()
      expect(state.query).toBe('')
      expect(state.results).toEqual([])
      expect(state.isSearching).toBe(false)
      expect(state.isOpen).toBe(false)
      expect(state.selectedIndex).toBe(-1)
    })
  })

  describe('setQuery', () => {
    it('updates query string', () => {
      useSearchStore.getState().setQuery('test')
      expect(useSearchStore.getState().query).toBe('test')
    })
  })

  describe('search', () => {
    it('returns track results from API', async () => {
      vi.mocked(apiClient.search).mockResolvedValueOnce([
        { trackId: 1, title: 'Song A', artist: 'Artist 1', album: 'Album 1', trackNumber: 1, discNumber: 1, lossless: true, relevanceScore: 1 },
      ])

      await useSearchStore.getState().search('song')

      const state = useSearchStore.getState()
      expect(state.results).toHaveLength(1)
      expect(state.results[0]).toEqual({
        id: 1,
        type: 'track',
        title: 'Song A',
        subtitle: 'Artist 1 — Album 1',
      })
      expect(state.isOpen).toBe(true)
      expect(state.isSearching).toBe(false)
    })

    it('includes audiobook results from client-side filter', async () => {
      vi.mocked(apiClient.search).mockResolvedValueOnce([])
      vi.mocked(useAudiobookStore.getState).mockReturnValue({
        audiobooks: [
          { id: 10, title: 'The Hobbit', authorId: 1, metadata: { narrator: 'Andy Serkis', genres: [], narrators: ['Andy Serkis'], isAbridged: false }, year: 1937, monitored: true, qualityProfileId: 1, added: '' },
        ],
        authors: [{ id: 1, name: 'J.R.R. Tolkien', monitored: true, qualityProfileId: 1, added: '' }],
      } as unknown as ReturnType<typeof useAudiobookStore.getState>)

      await useSearchStore.getState().search('hobbit')

      const state = useSearchStore.getState()
      expect(state.results).toHaveLength(1)
      expect(state.results[0]).toEqual({
        id: 10,
        type: 'audiobook',
        title: 'The Hobbit',
        subtitle: 'J.R.R. Tolkien',
      })
    })

    it('includes podcast results from client-side filter', async () => {
      vi.mocked(apiClient.search).mockResolvedValueOnce([])
      vi.mocked(usePodcastStore.getState).mockReturnValue({
        shows: [
          { id: 20, title: 'Lex Fridman Podcast', author: 'Lex Fridman', feedUrl: '', episodeCount: 400, monitored: true, monitorNewEpisodes: true, qualityProfileId: 1, added: '' },
        ],
      } as unknown as ReturnType<typeof usePodcastStore.getState>)

      await useSearchStore.getState().search('fridman')

      const state = useSearchStore.getState()
      expect(state.results).toHaveLength(1)
      expect(state.results[0].type).toBe('podcast')
      expect(state.results[0].title).toBe('Lex Fridman Podcast')
    })

    it('clears results for empty query', async () => {
      useSearchStore.setState({ results: [{ id: 1, type: 'track', title: 'X', subtitle: '' }], isOpen: true })

      await useSearchStore.getState().search('')

      expect(useSearchStore.getState().results).toEqual([])
      expect(useSearchStore.getState().isOpen).toBe(false)
    })

    it('caps results at 20', async () => {
      const manyTracks = Array.from({ length: 25 }, (_, i) => ({
        trackId: i, title: `Song ${i}`, artist: 'A', album: 'B', trackNumber: 1, discNumber: 1, lossless: true, relevanceScore: 1,
      }))
      vi.mocked(apiClient.search).mockResolvedValueOnce(manyTracks)

      await useSearchStore.getState().search('song')

      expect(useSearchStore.getState().results.length).toBeLessThanOrEqual(20)
    })

    it('handles API error gracefully', async () => {
      vi.mocked(apiClient.search).mockRejectedValueOnce(new Error('Network'))

      await useSearchStore.getState().search('test')

      const state = useSearchStore.getState()
      expect(state.results).toEqual([])
      expect(state.isSearching).toBe(false)
    })

    it('resets selectedIndex on new search', async () => {
      useSearchStore.setState({ selectedIndex: 3 })
      vi.mocked(apiClient.search).mockResolvedValueOnce([
        { trackId: 1, title: 'X', artist: 'A', trackNumber: 1, discNumber: 1, lossless: true, relevanceScore: 1 },
      ])

      await useSearchStore.getState().search('x')

      expect(useSearchStore.getState().selectedIndex).toBe(-1)
    })
  })

  describe('setOpen', () => {
    it('toggles dropdown and resets selectedIndex', () => {
      useSearchStore.setState({ selectedIndex: 5 })
      useSearchStore.getState().setOpen(true)
      expect(useSearchStore.getState().isOpen).toBe(true)
      expect(useSearchStore.getState().selectedIndex).toBe(-1)
    })
  })

  describe('setSelectedIndex', () => {
    it('updates selection', () => {
      useSearchStore.getState().setSelectedIndex(2)
      expect(useSearchStore.getState().selectedIndex).toBe(2)
    })
  })

  describe('clear', () => {
    it('resets all state', () => {
      useSearchStore.setState({
        query: 'test',
        results: [{ id: 1, type: 'track', title: 'X', subtitle: '' }],
        isSearching: true,
        isOpen: true,
        selectedIndex: 3,
      })

      useSearchStore.getState().clear()

      const state = useSearchStore.getState()
      expect(state.query).toBe('')
      expect(state.results).toEqual([])
      expect(state.isSearching).toBe(false)
      expect(state.isOpen).toBe(false)
      expect(state.selectedIndex).toBe(-1)
    })
  })
})
