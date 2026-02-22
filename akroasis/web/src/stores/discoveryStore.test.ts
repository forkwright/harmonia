import { describe, it, expect, beforeEach, vi } from 'vitest'
import { useDiscoveryStore } from './discoveryStore'
import { apiClient } from '../api/client'
import type { PlaybackSession, HistoryEntry, PagedHistory, Track, PagedResult } from '../types'

vi.mock('../api/client', () => ({
  apiClient: {
    getSessions: vi.fn(),
    getHistory: vi.fn(),
    getTracks: vi.fn(),
  },
}))

const mockSessions: PlaybackSession[] = [
  {
    id: 1,
    sessionId: 'sess-001',
    mediaItemId: 201,
    userId: 'default',
    deviceName: 'Desktop Chrome',
    deviceType: 'browser',
    startedAt: new Date(Date.now() - 3600000).toISOString(),
    endedAt: new Date(Date.now() - 1800000).toISOString(),
    startPositionMs: 0,
    endPositionMs: 1800000,
    durationMs: 1800000,
    isActive: false,
  },
  {
    id: 2,
    sessionId: 'sess-002',
    mediaItemId: 203,
    userId: 'default',
    deviceName: 'Mobile Safari',
    deviceType: 'mobile',
    startedAt: new Date(Date.now() - 7200000).toISOString(),
    endedAt: new Date(Date.now() - 5400000).toISOString(),
    startPositionMs: 0,
    endPositionMs: 1800000,
    durationMs: 1800000,
    isActive: true,
  },
]

const mockHistoryEntries: HistoryEntry[] = [
  {
    id: 101,
    mediaItemId: 201,
    mediaType: 3,
    sourceTitle: 'The Way of Kings',
    quality: { quality: { id: 7, name: 'FLAC' }, revision: { version: 1, real: 0 } },
    date: new Date(Date.now() - 86400000).toISOString(),
    eventType: 1,
    data: {},
  },
  {
    id: 102,
    mediaItemId: 203,
    mediaType: 3,
    sourceTitle: 'Dune',
    quality: { quality: { id: 7, name: 'FLAC' }, revision: { version: 1, real: 0 } },
    date: new Date(Date.now() - 172800000).toISOString(),
    eventType: 3,
    data: {},
  },
]

const mockPagedHistory: PagedHistory = {
  page: 1,
  pageSize: 20,
  sortKey: 'date',
  sortDirection: 'descending',
  totalRecords: 2,
  records: mockHistoryEntries,
}

const mockTracks: Track[] = [
  {
    id: 201,
    title: 'Lateralus',
    artist: 'Tool',
    album: 'Lateralus',
    duration: 563,
    fileSize: 45000000,
    format: 'FLAC',
    bitrate: 1411,
    sampleRate: 44100,
    bitDepth: 16,
    channels: 2,
  },
  {
    id: 203,
    title: 'Echoes',
    artist: 'Pink Floyd',
    album: 'Meddle',
    duration: 1411,
    fileSize: 98000000,
    format: 'FLAC',
    bitrate: 1411,
    sampleRate: 44100,
    bitDepth: 24,
    channels: 2,
  },
]

const mockPagedTracks: PagedResult<Track> = {
  items: mockTracks,
  page: 1,
  pageSize: 50,
  totalCount: 2,
}

function resetStore() {
  useDiscoveryStore.setState({
    sessions: [],
    recentHistory: [],
    tracks: [],
    isLoading: false,
    error: null,
  })
}

describe('discoveryStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  describe('initial state', () => {
    it('starts with empty sessions, history, and tracks', () => {
      const state = useDiscoveryStore.getState()
      expect(state.sessions).toEqual([])
      expect(state.recentHistory).toEqual([])
      expect(state.tracks).toEqual([])
      expect(state.isLoading).toBe(false)
      expect(state.error).toBeNull()
    })
  })

  describe('fetchSessions', () => {
    it('loads sessions and clears loading state', async () => {
      vi.mocked(apiClient.getSessions).mockResolvedValueOnce(mockSessions)

      await useDiscoveryStore.getState().fetchSessions()

      const state = useDiscoveryStore.getState()
      expect(state.sessions).toEqual(mockSessions)
      expect(state.isLoading).toBe(false)
      expect(state.error).toBeNull()
    })

    it('sets isLoading true during fetch', async () => {
      let resolvePromise!: (value: PlaybackSession[]) => void
      const pending = new Promise<PlaybackSession[]>((resolve) => {
        resolvePromise = resolve
      })
      vi.mocked(apiClient.getSessions).mockReturnValueOnce(pending)

      const fetchPromise = useDiscoveryStore.getState().fetchSessions()
      expect(useDiscoveryStore.getState().isLoading).toBe(true)

      resolvePromise(mockSessions)
      await fetchPromise

      expect(useDiscoveryStore.getState().isLoading).toBe(false)
    })

    it('sets error on failure', async () => {
      vi.mocked(apiClient.getSessions).mockRejectedValueOnce(new Error('Network error'))

      await useDiscoveryStore.getState().fetchSessions()

      const state = useDiscoveryStore.getState()
      expect(state.sessions).toEqual([])
      expect(state.error).toBe('Network error')
      expect(state.isLoading).toBe(false)
    })

    it('uses fallback message for non-Error rejections', async () => {
      vi.mocked(apiClient.getSessions).mockRejectedValueOnce('something bad')

      await useDiscoveryStore.getState().fetchSessions()

      expect(useDiscoveryStore.getState().error).toBe('Failed to load sessions')
    })

    it('clears previous error on new fetch', async () => {
      useDiscoveryStore.setState({ error: 'old error' })
      vi.mocked(apiClient.getSessions).mockResolvedValueOnce(mockSessions)

      await useDiscoveryStore.getState().fetchSessions()

      expect(useDiscoveryStore.getState().error).toBeNull()
    })
  })

  describe('fetchHistory', () => {
    it('loads history records and clears loading state', async () => {
      vi.mocked(apiClient.getHistory).mockResolvedValueOnce(mockPagedHistory)

      await useDiscoveryStore.getState().fetchHistory()

      const state = useDiscoveryStore.getState()
      expect(state.recentHistory).toEqual(mockHistoryEntries)
      expect(state.isLoading).toBe(false)
      expect(state.error).toBeNull()
    })

    it('calls getHistory with page 1 and pageSize 20', async () => {
      vi.mocked(apiClient.getHistory).mockResolvedValueOnce(mockPagedHistory)

      await useDiscoveryStore.getState().fetchHistory()

      expect(apiClient.getHistory).toHaveBeenCalledWith(1, 20)
    })

    it('sets isLoading true during fetch', async () => {
      let resolvePromise!: (value: PagedHistory) => void
      const pending = new Promise<PagedHistory>((resolve) => {
        resolvePromise = resolve
      })
      vi.mocked(apiClient.getHistory).mockReturnValueOnce(pending)

      const fetchPromise = useDiscoveryStore.getState().fetchHistory()
      expect(useDiscoveryStore.getState().isLoading).toBe(true)

      resolvePromise(mockPagedHistory)
      await fetchPromise

      expect(useDiscoveryStore.getState().isLoading).toBe(false)
    })

    it('sets error on failure', async () => {
      vi.mocked(apiClient.getHistory).mockRejectedValueOnce(new Error('Timeout'))

      await useDiscoveryStore.getState().fetchHistory()

      const state = useDiscoveryStore.getState()
      expect(state.recentHistory).toEqual([])
      expect(state.error).toBe('Timeout')
      expect(state.isLoading).toBe(false)
    })

    it('uses fallback message for non-Error rejections', async () => {
      vi.mocked(apiClient.getHistory).mockRejectedValueOnce(42)

      await useDiscoveryStore.getState().fetchHistory()

      expect(useDiscoveryStore.getState().error).toBe('Failed to load history')
    })

    it('clears previous error on new fetch', async () => {
      useDiscoveryStore.setState({ error: 'stale error' })
      vi.mocked(apiClient.getHistory).mockResolvedValueOnce(mockPagedHistory)

      await useDiscoveryStore.getState().fetchHistory()

      expect(useDiscoveryStore.getState().error).toBeNull()
    })
  })

  describe('fetchTracks', () => {
    it('loads tracks and clears loading state', async () => {
      vi.mocked(apiClient.getTracks).mockResolvedValueOnce(mockPagedTracks)

      await useDiscoveryStore.getState().fetchTracks()

      const state = useDiscoveryStore.getState()
      expect(state.tracks).toEqual(mockTracks)
      expect(state.isLoading).toBe(false)
      expect(state.error).toBeNull()
    })

    it('sets isLoading true during fetch', async () => {
      let resolvePromise!: (value: PagedResult<Track>) => void
      const pending = new Promise<PagedResult<Track>>((resolve) => {
        resolvePromise = resolve
      })
      vi.mocked(apiClient.getTracks).mockReturnValueOnce(pending)

      const fetchPromise = useDiscoveryStore.getState().fetchTracks()
      expect(useDiscoveryStore.getState().isLoading).toBe(true)

      resolvePromise(mockPagedTracks)
      await fetchPromise

      expect(useDiscoveryStore.getState().isLoading).toBe(false)
    })

    it('sets error on failure', async () => {
      vi.mocked(apiClient.getTracks).mockRejectedValueOnce(new Error('Connection refused'))

      await useDiscoveryStore.getState().fetchTracks()

      const state = useDiscoveryStore.getState()
      expect(state.tracks).toEqual([])
      expect(state.error).toBe('Connection refused')
      expect(state.isLoading).toBe(false)
    })

    it('uses fallback message for non-Error rejections', async () => {
      vi.mocked(apiClient.getTracks).mockRejectedValueOnce(undefined)

      await useDiscoveryStore.getState().fetchTracks()

      expect(useDiscoveryStore.getState().error).toBe('Failed to load tracks')
    })
  })

  describe('fetchAll', () => {
    it('fetches sessions, history, and tracks in parallel', async () => {
      vi.mocked(apiClient.getSessions).mockResolvedValueOnce(mockSessions)
      vi.mocked(apiClient.getHistory).mockResolvedValueOnce(mockPagedHistory)
      vi.mocked(apiClient.getTracks).mockResolvedValueOnce(mockPagedTracks)

      await useDiscoveryStore.getState().fetchAll()

      const state = useDiscoveryStore.getState()
      expect(state.sessions).toEqual(mockSessions)
      expect(state.recentHistory).toEqual(mockHistoryEntries)
      expect(state.tracks).toEqual(mockTracks)
      expect(state.isLoading).toBe(false)
      expect(state.error).toBeNull()
    })

    it('sets isLoading true during fetch', async () => {
      let resolve!: () => void
      const gate = new Promise<void>((r) => { resolve = r })

      vi.mocked(apiClient.getSessions).mockImplementation(() => gate.then(() => mockSessions))
      vi.mocked(apiClient.getHistory).mockImplementation(() => gate.then(() => mockPagedHistory))
      vi.mocked(apiClient.getTracks).mockImplementation(() => gate.then(() => mockPagedTracks))

      const fetchPromise = useDiscoveryStore.getState().fetchAll()
      expect(useDiscoveryStore.getState().isLoading).toBe(true)

      resolve()
      await fetchPromise

      expect(useDiscoveryStore.getState().isLoading).toBe(false)
    })

    it('sets error if any request fails', async () => {
      vi.mocked(apiClient.getSessions).mockResolvedValueOnce(mockSessions)
      vi.mocked(apiClient.getHistory).mockRejectedValueOnce(new Error('History unavailable'))
      vi.mocked(apiClient.getTracks).mockResolvedValueOnce(mockTracks)

      await useDiscoveryStore.getState().fetchAll()

      const state = useDiscoveryStore.getState()
      expect(state.error).toBe('History unavailable')
      expect(state.isLoading).toBe(false)
    })

    it('uses fallback message for non-Error rejections', async () => {
      vi.mocked(apiClient.getSessions).mockRejectedValueOnce(null)
      vi.mocked(apiClient.getHistory).mockResolvedValueOnce(mockPagedHistory)
      vi.mocked(apiClient.getTracks).mockResolvedValueOnce(mockTracks)

      await useDiscoveryStore.getState().fetchAll()

      expect(useDiscoveryStore.getState().error).toBe('Failed to load discovery data')
    })

    it('calls all three API methods', async () => {
      vi.mocked(apiClient.getSessions).mockResolvedValueOnce([])
      vi.mocked(apiClient.getHistory).mockResolvedValueOnce({ ...mockPagedHistory, records: [] })
      vi.mocked(apiClient.getTracks).mockResolvedValueOnce([])

      await useDiscoveryStore.getState().fetchAll()

      expect(apiClient.getSessions).toHaveBeenCalledOnce()
      expect(apiClient.getHistory).toHaveBeenCalledWith(1, 20)
      expect(apiClient.getTracks).toHaveBeenCalledOnce()
    })
  })
})
