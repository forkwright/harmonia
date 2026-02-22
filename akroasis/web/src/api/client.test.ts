import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { apiClient, getStreamUrl, getCoverArtUrl } from './client'
import type { Track, Album, Artist, AuthResponse } from '../types'

globalThis.fetch = vi.fn()

const localStorageMock = (() => {
  let store: Record<string, string> = {}

  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value
    },
    removeItem: (key: string) => {
      delete store[key]
    },
    clear: () => {
      store = {}
    }
  }
})()

Object.defineProperty(globalThis, 'localStorage', {
  value: localStorageMock
})

describe('ApiClient', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorageMock.clear()
    apiClient.clearAuth()
  })

  afterEach(() => {
    localStorageMock.clear()
  })

  describe('Initialization', () => {
    it('should use singleton instance', () => {
      expect(apiClient).toBeDefined()
    })

    it('should load server URL from localStorage', () => {
      localStorageMock.setItem('serverUrl', 'http://localhost:5000')
      apiClient.setServerUrl('http://localhost:5000')

      expect(localStorageMock.getItem('serverUrl')).toBe('http://localhost:5000')
    })

    it('should strip trailing slash from baseUrl', () => {
      apiClient.setServerUrl('http://localhost:5000/')

      const streamUrl = apiClient.getStreamUrl(1)
      expect(streamUrl).toBe('http://localhost:5000/api/v3/stream/1')
    })
  })

  describe('Configuration', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
    })

    it('should set server URL and save to localStorage', () => {
      apiClient.setServerUrl('http://newserver:5000')

      expect(localStorageMock.getItem('serverUrl')).toBe('http://newserver:5000')
    })

    it('should strip trailing slash when setting server URL', () => {
      apiClient.setServerUrl('http://newserver:5000/')

      const streamUrl = apiClient.getStreamUrl(1)
      expect(streamUrl).toBe('http://newserver:5000/api/v3/stream/1')
    })

    it('should set tokens and save to localStorage', () => {
      apiClient.setTokens('access-tok', 'refresh-tok')

      expect(localStorageMock.getItem('accessToken')).toBe('access-tok')
      expect(localStorageMock.getItem('refreshToken')).toBe('refresh-tok')
    })

    it('should clear authentication', () => {
      apiClient.setTokens('access', 'refresh')
      expect(localStorageMock.getItem('accessToken')).toBe('access')

      apiClient.clearAuth()
      expect(localStorageMock.getItem('accessToken')).toBeNull()
      expect(localStorageMock.getItem('refreshToken')).toBeNull()
    })
  })

  describe('Authentication', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
    })

    it('should login successfully with new auth response shape', async () => {
      const mockAuthResponse: AuthResponse = {
        accessToken: 'new-access-token',
        refreshToken: 'new-refresh-token',
        user: {
          id: 1,
          username: 'testuser',
          displayName: 'Test User',
          email: 'test@localhost',
          role: 'admin',
          authenticationMethod: 'forms',
          isActive: true,
          createdAt: '2025-01-01T00:00:00Z',
        },
      }

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockAuthResponse
      })

      const result = await apiClient.login('testuser', 'password123')

      expect(result).toEqual(mockAuthResponse)
      expect(result.accessToken).toBe('new-access-token')
      expect(result.user.username).toBe('testuser')
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/auth/login',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ username: 'testuser', password: 'password123' })
        })
      )
    })

    it('should include Bearer token in request headers when set', async () => {
      apiClient.setTokens('test-access-token', 'test-refresh-token')

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => []
      })

      await apiClient.getArtists()

      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/artists',
        expect.objectContaining({
          headers: expect.objectContaining({
            'Authorization': 'Bearer test-access-token'
          })
        })
      )
    })

    it('should not include auth header when no token set', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => []
      })

      await apiClient.getArtists()

      const callHeaders = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].headers
      expect(callHeaders.Authorization).toBeUndefined()
    })

    it('should attempt refresh on 401 response', async () => {
      apiClient.setTokens('expired-token', 'valid-refresh')

      const refreshResponse: AuthResponse = {
        accessToken: 'new-access',
        refreshToken: 'new-refresh',
        user: {
          id: 1, username: 'admin', displayName: 'Admin',
          email: 'a@b.com', role: 'admin', authenticationMethod: 'forms',
          isActive: true, createdAt: '2025-01-01T00:00:00Z',
        },
      }

      ;(globalThis.fetch as ReturnType<typeof vi.fn>)
        .mockResolvedValueOnce({ ok: false, status: 401, statusText: 'Unauthorized', json: async () => ({ message: 'Token expired' }) })
        .mockResolvedValueOnce({ ok: true, status: 200, json: async () => refreshResponse })
        .mockResolvedValueOnce({ ok: true, status: 200, json: async () => [] })

      const result = await apiClient.getArtists()

      expect(result).toEqual([])
      expect(globalThis.fetch).toHaveBeenCalledTimes(3)
      expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[1][0]).toBe('http://localhost:5000/api/v3/auth/refresh')
    })

    it('should clear auth and call onLogout when refresh fails', async () => {
      const onLogout = vi.fn()
      apiClient.setOnLogout(onLogout)
      apiClient.setTokens('expired-token', 'expired-refresh')

      ;(globalThis.fetch as ReturnType<typeof vi.fn>)
        .mockResolvedValueOnce({ ok: false, status: 401, statusText: 'Unauthorized', json: async () => ({ message: 'Token expired' }) })
        .mockResolvedValueOnce({ ok: false, status: 401, statusText: 'Unauthorized', json: async () => ({ message: 'Refresh token expired' }) })

      await expect(apiClient.getArtists()).rejects.toThrow('Session expired')
      expect(onLogout).toHaveBeenCalledOnce()
      expect(localStorageMock.getItem('accessToken')).toBeNull()
    })

    it('should not attempt refresh when no refresh token available', async () => {
      apiClient.setTokens('expired-token', 'refresh')
      apiClient.clearAuth()

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        json: async () => ({ message: 'Invalid token' })
      })

      await expect(apiClient.getArtists()).rejects.toThrow('Invalid token')
      expect(globalThis.fetch).toHaveBeenCalledTimes(1)
    })

    it('should logout via server and clear tokens', async () => {
      apiClient.setTokens('tok', 'ref')

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 204,
        json: async () => undefined
      })

      await apiClient.logout()

      expect(localStorageMock.getItem('accessToken')).toBeNull()
      expect(localStorageMock.getItem('refreshToken')).toBeNull()
    })
  })

  describe('Library Data Fetching', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
      apiClient.setTokens('test-token', 'test-refresh')
    })

    it('should fetch artists', async () => {
      const mockArtists: Artist[] = [
        { id: 1, name: 'Artist 1', albumCount: 5, trackCount: 50 },
        { id: 2, name: 'Artist 2', albumCount: 3, trackCount: 30 }
      ]

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockArtists
      })

      const result = await apiClient.getArtists()

      expect(result).toEqual(mockArtists)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/artists',
        expect.any(Object)
      )
    })

    it('should fetch all albums', async () => {
      const mockAlbums: Album[] = [
        { id: 1, title: 'Album 1', artist: 'Artist 1', year: 2020, trackCount: 10, duration: 1800 },
        { id: 2, title: 'Album 2', artist: 'Artist 2', year: 2021, trackCount: 12, duration: 2400 }
      ]

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockAlbums
      })

      const result = await apiClient.getAlbums()

      expect(result).toEqual(mockAlbums)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/albums',
        expect.any(Object)
      )
    })

    it('should fetch albums by artist ID', async () => {
      const mockAlbums: Album[] = [
        { id: 1, title: 'Album 1', artist: 'Artist 1', year: 2020, trackCount: 10, duration: 1800 }
      ]

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockAlbums
      })

      const result = await apiClient.getAlbums(1)

      expect(result).toEqual(mockAlbums)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/artists/1/albums',
        expect.any(Object)
      )
    })

    it('should fetch all tracks', async () => {
      const mockTracks: Track[] = [
        { id: 1, title: 'Track 1', artist: 'Artist 1', album: 'Album 1', duration: 180, fileSize: 5000000, format: 'FLAC', bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2 },
      ]

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockTracks
      })

      const result = await apiClient.getTracks()

      expect(result).toEqual(mockTracks)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/tracks',
        expect.any(Object)
      )
    })

    it('should fetch tracks by album ID', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => []
      })

      await apiClient.getTracks(1)

      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/albums/1/tracks',
        expect.any(Object)
      )
    })

    it('should fetch single track by ID', async () => {
      const mockTrack: Track = {
        id: 1, title: 'Track 1', artist: 'Artist 1', album: 'Album 1', duration: 180,
        fileSize: 5000000, format: 'FLAC', bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2
      }

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => mockTrack
      })

      const result = await apiClient.getTrack(1)

      expect(result).toEqual(mockTrack)
    })
  })

  describe('URL Building', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
    })

    it('should build stream URL', () => {
      expect(apiClient.getStreamUrl(42)).toBe('http://localhost:5000/api/v3/stream/42')
    })

    it('should build stream URL via helper', () => {
      expect(getStreamUrl(42)).toContain('/api/v3/stream/42')
    })

    it('should build cover art URL without size', () => {
      expect(apiClient.getCoverArtUrl(42)).toBe('http://localhost:5000/api/v3/mediacover/track/42/poster.jpg')
    })

    it('should build cover art URL with size', () => {
      expect(apiClient.getCoverArtUrl(42, 300)).toBe('http://localhost:5000/api/v3/mediacover/track/42/poster.jpg?width=300&height=300')
    })

    it('should build cover art URL via helper', () => {
      expect(getCoverArtUrl(42, 300)).toContain('/api/v3/mediacover/track/42/poster.jpg?width=300&height=300')
    })
  })

  describe('Sessions API', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
      apiClient.setTokens('test-token', 'test-refresh')
    })

    it('should fetch all sessions', async () => {
      const sessions = [{ id: 1, sessionId: 'sess-1' }]
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => sessions
      })

      const result = await apiClient.getSessions()
      expect(result).toEqual(sessions)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/sessions',
        expect.any(Object)
      )
    })

    it('should fetch session by ID', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => ({ sessionId: 'sess-1' })
      })

      await apiClient.getSession('sess-1')
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/sessions/sess-1',
        expect.any(Object)
      )
    })

    it('should fetch sessions by media item', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => []
      })

      await apiClient.getMediaSessions(42)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/sessions/media/42',
        expect.any(Object)
      )
    })

    it('should create a session', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 201, json: async () => ({ id: 1 })
      })

      await apiClient.createSession({
        sessionId: 'new', mediaItemId: 1, userId: 'admin',
        deviceName: 'Web', deviceType: 'web',
        startedAt: '2026-01-01T00:00:00Z', startPositionMs: 0,
        durationMs: 0, isActive: true,
      })

      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/sessions',
        expect.objectContaining({ method: 'POST' })
      )
    })

    it('should update a session', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => ({})
      })

      await apiClient.updateSession('sess-1', { isActive: false })
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/sessions/sess-1',
        expect.objectContaining({ method: 'PUT' })
      )
    })

    it('should delete a session', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 204, json: async () => undefined
      })

      await apiClient.deleteSession('sess-1')
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/sessions/sess-1',
        expect.objectContaining({ method: 'DELETE' })
      )
    })
  })

  describe('History API', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
      apiClient.setTokens('test-token', 'test-refresh')
    })

    it('should fetch paged history', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => ({ page: 1, pageSize: 50, totalRecords: 0, records: [] })
      })

      const result = await apiClient.getHistory(1, 50)
      expect(result.records).toEqual([])
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/history?page=1&pageSize=50',
        expect.any(Object)
      )
    })

    it('should fetch history since date', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => []
      })

      await apiClient.getHistorySince('2026-01-01')
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/history/since?date=2026-01-01',
        expect.any(Object)
      )
    })

    it('should fetch media item history', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => []
      })

      await apiClient.getMediaItemHistory(42)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/history/mediaitem/42',
        expect.any(Object)
      )
    })

    it('should delete a history entry', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 204, json: async () => undefined
      })

      await apiClient.deleteHistoryEntry(5)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/history/5',
        expect.objectContaining({ method: 'DELETE' })
      )
    })
  })

  describe('Podcasts API', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
      apiClient.setTokens('test-token', 'test-refresh')
    })

    it('should fetch paged podcasts', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => ({ items: [], page: 1, pageSize: 50, totalCount: 0 })
      })

      const result = await apiClient.getPodcasts(1, 50)
      expect(result.items).toEqual([])
    })

    it('should fetch single podcast', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => ({ id: 1, title: 'Test Pod' })
      })

      const result = await apiClient.getPodcast(1)
      expect(result.title).toBe('Test Pod')
    })

    it('should add a podcast', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 201, json: async () => ({ id: 1 })
      })

      await apiClient.addPodcast({
        title: 'New Pod', feedUrl: 'https://example.com/feed',
        monitored: true, monitorNewEpisodes: true, qualityProfileId: 1,
      } as Parameters<typeof apiClient.addPodcast>[0])

      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/podcasts',
        expect.objectContaining({ method: 'POST' })
      )
    })

    it('should update a podcast', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => ({ id: 1 })
      })

      await apiClient.updatePodcast(1, { monitored: false })
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/podcasts/1',
        expect.objectContaining({ method: 'PUT' })
      )
    })

    it('should delete a podcast', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 204, json: async () => undefined
      })

      await apiClient.deletePodcast(1)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/podcasts/1',
        expect.objectContaining({ method: 'DELETE' })
      )
    })

    it('should fetch podcast episodes', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => [{ id: 1, title: 'EP 1' }]
      })

      const result = await apiClient.getPodcastEpisodes(1)
      expect(result).toHaveLength(1)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/podcasts/1/episodes',
        expect.any(Object)
      )
    })

    it('should fetch single episode', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true, status: 200, json: async () => ({ id: 5, title: 'EP 5' })
      })

      const result = await apiClient.getPodcastEpisode(5)
      expect(result.title).toBe('EP 5')
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/podcasts/episodes/5',
        expect.any(Object)
      )
    })
  })

  describe('Error Handling', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
    })

    it('should handle HTTP errors with JSON error message', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 403,
        statusText: 'Forbidden',
        json: async () => ({ message: 'Access denied' })
      })

      await expect(apiClient.getArtists()).rejects.toThrow('Access denied')
    })

    it('should handle HTTP errors without JSON body', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        json: async () => {
          throw new Error('Invalid JSON')
        }
      })

      await expect(apiClient.getArtists()).rejects.toThrow('HTTP 500: Internal Server Error')
    })

    it('should handle network errors', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
        new Error('Network error')
      )

      await expect(apiClient.getArtists()).rejects.toThrow('Network error')
    })

    it('should handle authentication errors on login', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 403,
        statusText: 'Forbidden',
        json: async () => ({ message: 'Invalid credentials' })
      })

      await expect(apiClient.login('user', 'wrong')).rejects.toThrow('Invalid credentials')
    })

    it('should handle 204 No Content responses', async () => {
      apiClient.setTokens('tok', 'ref')

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        status: 204,
      })

      const result = await apiClient.deleteSession('sess-1')
      expect(result).toBeUndefined()
    })
  })
})
