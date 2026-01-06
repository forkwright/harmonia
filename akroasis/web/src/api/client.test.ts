import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { apiClient, getStreamUrl, getCoverArtUrl } from './client'
import type { Track, Album, Artist, AuthResponse } from '../types'

// Mock fetch globally
globalThis.fetch = vi.fn()

// Mock localStorage
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
    // Reset apiClient state
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

    it('should load API key from localStorage', () => {
      localStorageMock.setItem('apiKey', 'test-api-key')
      apiClient.setApiKey('test-api-key')

      expect(localStorageMock.getItem('apiKey')).toBe('test-api-key')
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

    it('should set API key and save to localStorage', () => {
      apiClient.setApiKey('new-api-key')

      expect(localStorageMock.getItem('apiKey')).toBe('new-api-key')
    })

    it('should clear authentication', () => {
      apiClient.setApiKey('test-key')
      expect(localStorageMock.getItem('apiKey')).toBe('test-key')

      apiClient.clearAuth()
      expect(localStorageMock.getItem('apiKey')).toBeNull()
    })
  })

  describe('Authentication', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
    })

    it('should login successfully', async () => {
      const mockAuthResponse: AuthResponse = {
        token: 'new-api-token',
        expiresIn: 3600
      }

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => mockAuthResponse
      })

      const result = await apiClient.login('testuser', 'password123')

      expect(result).toEqual(mockAuthResponse)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/auth/login',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ username: 'testuser', password: 'password123' })
        })
      )
    })

    it('should include API key in request headers when set', async () => {
      apiClient.setApiKey('test-api-key')

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => []
      })

      await apiClient.getArtists()

      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/artists',
        expect.objectContaining({
          headers: expect.objectContaining({
            'X-Api-Key': 'test-api-key'
          })
        })
      )
    })
  })

  describe('Library Data Fetching', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
      apiClient.setApiKey('test-key')
    })

    it('should fetch artists', async () => {
      const mockArtists: Artist[] = [
        { id: 1, name: 'Artist 1', albumCount: 5, trackCount: 50 },
        { id: 2, name: 'Artist 2', albumCount: 3, trackCount: 30 }
      ]

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
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
        { id: 2, title: 'Track 2', artist: 'Artist 2', album: 'Album 2', duration: 200, fileSize: 6000000, format: 'FLAC', bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2 }
      ]

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
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
      const mockTracks: Track[] = [
        { id: 1, title: 'Track 1', artist: 'Artist 1', album: 'Album 1', duration: 180, fileSize: 5000000, format: 'FLAC', bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2 }
      ]

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => mockTracks
      })

      const result = await apiClient.getTracks(1)

      expect(result).toEqual(mockTracks)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/albums/1/tracks',
        expect.any(Object)
      )
    })

    it('should fetch single track by ID', async () => {
      const mockTrack: Track = {
        id: 1,
        title: 'Track 1',
        artist: 'Artist 1',
        album: 'Album 1',
        duration: 180,
        fileSize: 5000000,
        format: 'FLAC',
        bitrate: 1411,
        sampleRate: 44100,
        bitDepth: 16,
        channels: 2
      }

      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: true,
        json: async () => mockTrack
      })

      const result = await apiClient.getTrack(1)

      expect(result).toEqual(mockTrack)
      expect(globalThis.fetch).toHaveBeenCalledWith(
        'http://localhost:5000/api/v3/tracks/1',
        expect.any(Object)
      )
    })
  })

  describe('URL Building', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
    })

    it('should build stream URL', () => {
      const streamUrl = apiClient.getStreamUrl(42)

      expect(streamUrl).toBe('http://localhost:5000/api/v3/stream/42')
    })

    it('should build stream URL via helper', () => {
      const streamUrl = getStreamUrl(42)

      expect(streamUrl).toContain('/api/v3/stream/42')
    })

    it('should build cover art URL without size', () => {
      const coverUrl = apiClient.getCoverArtUrl(42)

      expect(coverUrl).toBe('http://localhost:5000/api/v3/mediacover/track/42/poster.jpg')
    })

    it('should build cover art URL with size', () => {
      const coverUrl = apiClient.getCoverArtUrl(42, 300)

      expect(coverUrl).toBe('http://localhost:5000/api/v3/mediacover/track/42/poster.jpg?width=300&height=300')
    })

    it('should build cover art URL via helper', () => {
      const coverUrl = getCoverArtUrl(42, 300)

      expect(coverUrl).toContain('/api/v3/mediacover/track/42/poster.jpg?width=300&height=300')
    })
  })

  describe('Error Handling', () => {
    beforeEach(() => {
      apiClient.setServerUrl('http://localhost:5000')
    })

    it('should handle HTTP errors with JSON error message', async () => {
      ;(globalThis.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        json: async () => ({ message: 'Invalid API key' })
      })

      await expect(apiClient.getArtists()).rejects.toThrow('Invalid API key')
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
  })
})
