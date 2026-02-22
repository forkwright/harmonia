import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { apiClient } from './client'
import type { Author, Audiobook, Chapter, MediaProgress, ContinueItem, SearchResult } from '../types'

globalThis.fetch = vi.fn()

const localStorageMock = (() => {
  let store: Record<string, string> = {}
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value },
    removeItem: (key: string) => { delete store[key] },
    clear: () => { store = {} },
  }
})()

Object.defineProperty(globalThis, 'localStorage', { value: localStorageMock })

const mockFetch = (data: unknown, ok = true, status = 200, statusText = 'OK') => {
  vi.mocked(fetch).mockResolvedValueOnce({
    ok,
    status,
    statusText,
    json: async () => data,
  } as Response)
}

describe('ApiClient — audiobook and extended endpoints', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorageMock.clear()
    apiClient.clearAuth()
    apiClient.setServerUrl('http://localhost:5000')
    apiClient.setTokens('test-key', 'test-refresh')
  })

  afterEach(() => {
    localStorageMock.clear()
  })

  // -------------------------
  // Authors
  // -------------------------
  describe('getAuthors', () => {
    const mockPagedAuthors = {
      items: [
        { id: 1, name: 'J.R.R. Tolkien', monitored: true, qualityProfileId: 1, added: '2020-01-01' },
        { id: 2, name: 'Frank Herbert', monitored: true, qualityProfileId: 1, added: '2020-01-02' },
      ],
      page: 1,
      pageSize: 50,
      totalCount: 2,
    }

    it('fetches paged authors with defaults', async () => {
      mockFetch(mockPagedAuthors)

      const result = await apiClient.getAuthors()

      expect(result.items).toHaveLength(2)
      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/authors?page=1&pageSize=50',
        expect.any(Object)
      )
    })

    it('passes custom page and pageSize', async () => {
      mockFetch({ ...mockPagedAuthors, page: 2, pageSize: 10 })

      await apiClient.getAuthors(2, 10)

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/authors?page=2&pageSize=10',
        expect.any(Object)
      )
    })

    it('returns paged result structure', async () => {
      mockFetch(mockPagedAuthors)

      const result = await apiClient.getAuthors()

      expect(result.page).toBe(1)
      expect(result.pageSize).toBe(50)
      expect(result.totalCount).toBe(2)
    })
  })

  describe('getAuthor', () => {
    const mockAuthor: Author = {
      id: 1,
      name: 'J.R.R. Tolkien',
      monitored: true,
      qualityProfileId: 1,
      added: '2020-01-01',
    }

    it('fetches single author by id', async () => {
      mockFetch(mockAuthor)

      const result = await apiClient.getAuthor(1)

      expect(result).toEqual(mockAuthor)
      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/authors/1',
        expect.any(Object)
      )
    })
  })

  // -------------------------
  // Audiobooks
  // -------------------------
  describe('getAudiobooks', () => {
    const mockPagedAudiobooks = {
      items: [
        {
          id: 1,
          title: 'The Fellowship of the Ring',
          year: 1954,
          monitored: true,
          qualityProfileId: 1,
          added: '2021-01-01',
          authorId: 1,
          metadata: { genres: [], narrators: [], isAbridged: false, durationMinutes: 1200 },
        },
      ],
      page: 1,
      pageSize: 50,
      totalCount: 1,
    }

    it('fetches paged audiobooks', async () => {
      mockFetch(mockPagedAudiobooks)

      const result = await apiClient.getAudiobooks()

      expect(result.items).toHaveLength(1)
      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/audiobooks?page=1&pageSize=50',
        expect.any(Object)
      )
    })

    it('uses custom page and pageSize', async () => {
      mockFetch({ ...mockPagedAudiobooks, page: 3, pageSize: 20 })

      await apiClient.getAudiobooks(3, 20)

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/audiobooks?page=3&pageSize=20',
        expect.any(Object)
      )
    })
  })

  describe('getAudiobook', () => {
    const mockBook: Audiobook = {
      id: 5,
      title: 'Dune',
      year: 1965,
      monitored: true,
      qualityProfileId: 1,
      added: '2021-03-01',
      authorId: 2,
      metadata: { genres: ['Science Fiction'], narrators: ['Scott Brick'], isAbridged: false, durationMinutes: 960 },
    }

    it('fetches single audiobook by id', async () => {
      mockFetch(mockBook)

      const result = await apiClient.getAudiobook(5)

      expect(result).toEqual(mockBook)
      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/audiobooks/5',
        expect.any(Object)
      )
    })
  })

  describe('getAudiobooksByAuthor', () => {
    it('fetches audiobooks filtered by author id', async () => {
      mockFetch([{ id: 1, title: 'Test Book', year: 2000, monitored: true, qualityProfileId: 1, added: '', metadata: { genres: [], narrators: [], isAbridged: false } }])

      await apiClient.getAudiobooksByAuthor(3)

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/audiobooks/author/3',
        expect.any(Object)
      )
    })
  })

  describe('getAudiobooksBySeries', () => {
    it('fetches audiobooks by series id', async () => {
      mockFetch([])

      await apiClient.getAudiobooksBySeries(7)

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/audiobooks/series/7',
        expect.any(Object)
      )
    })
  })

  describe('getAudiobookCoverUrl', () => {
    it('builds cover URL without size', () => {
      const url = apiClient.getAudiobookCoverUrl(10)
      expect(url).toBe('http://localhost:5000/api/v3/mediacover/10/poster')
    })

    it('builds cover URL with size', () => {
      const url = apiClient.getAudiobookCoverUrl(10, 256)
      expect(url).toBe('http://localhost:5000/api/v3/mediacover/10/poster?width=256&height=256')
    })
  })

  // -------------------------
  // Chapters
  // -------------------------
  describe('getChapters', () => {
    const mockChapters: Chapter[] = [
      { title: 'Prologue', startTimeMs: 0, endTimeMs: 120000, index: 0 },
      { title: 'Chapter 1', startTimeMs: 120000, endTimeMs: 480000, index: 1 },
    ]

    it('fetches chapters for media file', async () => {
      mockFetch(mockChapters)

      const result = await apiClient.getChapters(42)

      expect(result).toEqual(mockChapters)
      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/chapters/42',
        expect.any(Object)
      )
    })

    it('returns array of chapter objects', async () => {
      mockFetch(mockChapters)

      const result = await apiClient.getChapters(1)

      expect(result[0]!.title).toBe('Prologue')
      expect(result[1]!.startTimeMs).toBe(120000)
    })
  })

  // -------------------------
  // Progress
  // -------------------------
  describe('getProgress', () => {
    const mockProgress: MediaProgress = {
      id: 1,
      mediaItemId: 5,
      userId: 'default',
      positionMs: 300000,
      totalDurationMs: 3600000,
      percentComplete: 8.33,
      lastPlayedAt: '2026-01-01T12:00:00Z',
      isComplete: false,
      createdAt: '2026-01-01T10:00:00Z',
      updatedAt: '2026-01-01T12:00:00Z',
    }

    it('fetches progress with default userId', async () => {
      mockFetch(mockProgress)

      await apiClient.getProgress(5)

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/progress/5?userId=default',
        expect.any(Object)
      )
    })

    it('fetches progress with custom userId', async () => {
      mockFetch(mockProgress)

      await apiClient.getProgress(5, 'user-abc')

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/progress/5?userId=user-abc',
        expect.any(Object)
      )
    })
  })

  describe('updateProgress', () => {
    const mockProgress: MediaProgress = {
      id: 2,
      mediaItemId: 5,
      userId: 'default',
      positionMs: 600000,
      totalDurationMs: 3600000,
      percentComplete: 16.67,
      lastPlayedAt: '2026-01-01T13:00:00Z',
      isComplete: false,
      createdAt: '2026-01-01T10:00:00Z',
      updatedAt: '2026-01-01T13:00:00Z',
    }

    it('posts progress update with correct body', async () => {
      mockFetch(mockProgress)

      await apiClient.updateProgress(5, 600000, 3600000)

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/progress',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ mediaItemId: 5, positionMs: 600000, totalDurationMs: 3600000, isComplete: false }),
        })
      )
    })

    it('posts with isComplete true when specified', async () => {
      mockFetch(mockProgress)

      await apiClient.updateProgress(5, 3600000, 3600000, true)

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/progress',
        expect.objectContaining({
          body: JSON.stringify({ mediaItemId: 5, positionMs: 3600000, totalDurationMs: 3600000, isComplete: true }),
        })
      )
    })
  })

  // -------------------------
  // Continue Listening
  // -------------------------
  describe('getContinueListening', () => {
    const mockItems: ContinueItem[] = [
      {
        mediaItemId: 1,
        title: 'Dune',
        mediaType: 'audiobook',
        positionMs: 300000,
        totalDurationMs: 57600000,
        percentComplete: 0.52,
        lastPlayedAt: '2026-01-10T20:00:00Z',
        coverUrl: 'http://localhost:5000/api/v3/mediacover/1/poster',
      },
    ]

    it('fetches continue listening with default limit', async () => {
      mockFetch(mockItems)

      await apiClient.getContinueListening()

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/continue?limit=20',
        expect.any(Object)
      )
    })

    it('uses custom limit parameter', async () => {
      mockFetch(mockItems)

      await apiClient.getContinueListening(5)

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/continue?limit=5',
        expect.any(Object)
      )
    })

    it('returns array of continue items', async () => {
      mockFetch(mockItems)

      const result = await apiClient.getContinueListening()

      expect(result).toEqual(mockItems)
      expect(result[0]!.title).toBe('Dune')
    })
  })

  // -------------------------
  // Search
  // -------------------------
  describe('search', () => {
    const mockResults: SearchResult[] = [
      {
        trackId: 1,
        title: 'Money',
        artist: 'Pink Floyd',
        album: 'The Dark Side of the Moon',
        trackNumber: 6,
        discNumber: 1,
        durationSeconds: 382,
        lossless: true,
        relevanceScore: 0.95,
      },
    ]

    it('searches with encoded query', async () => {
      mockFetch(mockResults)

      await apiClient.search('Pink Floyd')

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/search?q=Pink%20Floyd&limit=50',
        expect.any(Object)
      )
    })

    it('URL-encodes special characters in query', async () => {
      mockFetch(mockResults)

      await apiClient.search('AC/DC & more')

      const calledUrl = (fetch as ReturnType<typeof vi.fn>).mock.calls[0]![0] as string
      expect(calledUrl).toContain('q=AC%2FDC')
    })

    it('uses custom limit', async () => {
      mockFetch(mockResults)

      await apiClient.search('jazz', 10)

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/search?q=jazz&limit=10',
        expect.any(Object)
      )
    })

    it('returns search result array', async () => {
      mockFetch(mockResults)

      const result = await apiClient.search('money')

      expect(result).toEqual(mockResults)
      expect(result[0]!.title).toBe('Money')
    })
  })

  // -------------------------
  // Scrobble
  // -------------------------
  describe('scrobble', () => {
    it('posts scrobble entry to correct endpoint', async () => {
      vi.mocked(fetch).mockResolvedValueOnce({
        ok: true,
        status: 204,
        json: async () => null,
      } as unknown as Response)

      const entry = {
        artist: 'Pink Floyd',
        track: 'Money',
        album: 'The Dark Side of the Moon',
        timestamp: 1700000000,
        duration: 382,
      }

      await apiClient.scrobble(entry)

      expect(fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v1/scrobble',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(entry),
        })
      )
    })

    it('includes API key header in scrobble request', async () => {
      vi.mocked(fetch).mockResolvedValueOnce({
        ok: true,
        status: 204,
        json: async () => null,
      } as unknown as Response)

      await apiClient.scrobble({
        artist: 'Radiohead',
        track: 'Paranoid Android',
        album: 'OK Computer',
        timestamp: 1700000001,
        duration: 383,
      })

      expect(fetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({ 'Authorization': 'Bearer test-key' }),
        })
      )
    })

    it('throws on scrobble failure', async () => {
      vi.mocked(fetch).mockResolvedValueOnce({
        ok: false,
        status: 503,
        statusText: 'Service Unavailable',
        json: async () => ({ message: 'Scrobble service down' }),
      } as Response)

      await expect(
        apiClient.scrobble({ artist: 'a', track: 'b', album: 'c', timestamp: 0, duration: 60 })
      ).rejects.toThrow('Scrobble service down')
    })
  })

  // -------------------------
  // Request headers
  // -------------------------
  describe('request headers', () => {
    it('sends Content-Type application/json on all requests', async () => {
      mockFetch([])

      await apiClient.getArtists()

      expect(fetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: expect.objectContaining({ 'Content-Type': 'application/json' }),
        })
      )
    })

    it('omits Authorization header when no token set', async () => {
      apiClient.clearAuth()
      mockFetch([])

      await apiClient.getArtists()

      const headers = (fetch as ReturnType<typeof vi.fn>).mock.calls[0]![1]!.headers as Record<string, string>
      expect(headers['Authorization']).toBeUndefined()
    })
  })
})
