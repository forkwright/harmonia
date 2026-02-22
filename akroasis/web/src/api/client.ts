// Mouseion API client
import { logError, logWarn, logInfo } from '../utils/errorLogger'
import type {
  Track, Album, Artist, AuthResponse, ApiError,
  Audiobook, Author, Chapter, MediaProgress, ContinueItem,
  PagedResult, SearchResult, PendingScrobble,
  PlaybackSession, HistoryEntry, PagedHistory,
  PodcastShow, PodcastEpisode, Playlist,
  LibraryFacets, FilterRequest, FilterResponse,
} from '../types'

type LogoutCallback = () => void

class ApiClient {
  private baseUrl: string
  private accessToken: string | null = null
  private refreshTokenValue: string | null = null
  private refreshPromise: Promise<AuthResponse> | null = null
  private onLogout: LogoutCallback | null = null

  constructor(baseUrl: string = '') {
    const storedUrl = localStorage.getItem('serverUrl')
    const defaultUrl = ''  // Use Vite proxy in dev, same-origin in prod
    this.baseUrl = (baseUrl || storedUrl || defaultUrl).replace(/\/$/, '')

    const storedToken = localStorage.getItem('accessToken')
    if (storedToken) {
      this.accessToken = storedToken
    }
    const storedRefresh = localStorage.getItem('refreshToken')
    if (storedRefresh) {
      this.refreshTokenValue = storedRefresh
    }
  }

  setServerUrl(url: string) {
    this.baseUrl = url.replace(/\/$/, '')
    localStorage.setItem('serverUrl', this.baseUrl)
  }

  setTokens(accessToken: string, refreshToken: string) {
    this.accessToken = accessToken
    this.refreshTokenValue = refreshToken
    localStorage.setItem('accessToken', accessToken)
    localStorage.setItem('refreshToken', refreshToken)
  }

  setOnLogout(callback: LogoutCallback) {
    this.onLogout = callback
  }

  clearAuth() {
    this.accessToken = null
    this.refreshTokenValue = null
    this.refreshPromise = null
    localStorage.removeItem('accessToken')
    localStorage.removeItem('refreshToken')
    localStorage.removeItem('apiKey')
  }

  private async request<T>(endpoint: string, options: RequestInit = {}, skipAuth = false): Promise<T> {
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
      ...(this.accessToken && !skipAuth && { 'Authorization': `Bearer ${this.accessToken}` }),
      ...options.headers,
    }

    logInfo('api', `${options.method || 'GET'} ${endpoint}`)

    let response: Response
    try {
      response = await fetch(`${this.baseUrl}${endpoint}`, {
        ...options,
        headers,
      })
    } catch (err) {
      logError('api:network', `${options.method || 'GET'} ${endpoint} — network failure`, err)
      throw err
    }

    if (!response.ok && response.status !== 401) {
      logWarn('api:status', `${options.method || 'GET'} ${endpoint} → ${response.status} ${response.statusText}`)
    }

    if (response.status === 401 && !skipAuth && this.refreshTokenValue) {
      const refreshed = await this.tryRefresh()
      if (refreshed) {
        const retryHeaders: HeadersInit = {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.accessToken}`,
          ...options.headers,
        }
        const retryResponse = await fetch(`${this.baseUrl}${endpoint}`, {
          ...options,
          headers: retryHeaders,
        })
        if (!retryResponse.ok) {
          const error: ApiError = await retryResponse.json().catch(() => ({
            message: `HTTP ${retryResponse.status}: ${retryResponse.statusText}`,
          }))
          throw new Error(error.message)
        }
        if (retryResponse.status === 204) {
          return undefined as T
        }
        return retryResponse.json()
      }
      this.clearAuth()
      this.onLogout?.()
      throw new Error('Session expired')
    }

    if (!response.ok) {
      const error: ApiError = await response.json().catch(() => ({
        message: `HTTP ${response.status}: ${response.statusText}`,
      }))
      throw new Error(error.message)
    }

    if (response.status === 204) {
      return undefined as T
    }

    return response.json()
  }

  private async tryRefresh(): Promise<boolean> {
    if (!this.refreshTokenValue) return false

    if (this.refreshPromise) {
      try {
        await this.refreshPromise
        return true
      } catch {
        return false
      }
    }

    this.refreshPromise = this.request<AuthResponse>(
      '/api/v3/auth/refresh',
      {
        method: 'POST',
        body: JSON.stringify({ refreshToken: this.refreshTokenValue }),
      },
      true,
    )

    try {
      const result = await this.refreshPromise
      this.setTokens(result.accessToken, result.refreshToken)
      return true
    } catch {
      return false
    } finally {
      this.refreshPromise = null
    }
  }

  // --- Auth ---

  async login(username: string, password: string): Promise<AuthResponse> {
    return this.request<AuthResponse>('/api/v3/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }, true)
  }

  async logout(): Promise<void> {
    try {
      await this.request<void>('/api/v3/auth/logout', { method: 'POST' })
    } catch {
      // Best-effort server logout
    }
    this.clearAuth()
  }

  // --- Music ---

  async getArtists(page = 1, pageSize = 50): Promise<PagedResult<Artist>> {
    return this.request<PagedResult<Artist>>(`/api/v3/artists/music?page=${page}&pageSize=${pageSize}`)
  }

  async getAlbums(): Promise<PagedResult<Album>>
  async getAlbums(artistId: number): Promise<Album[]>
  async getAlbums(artistId?: number): Promise<Album[] | PagedResult<Album>> {
    if (artistId) {
      return this.request<Album[]>(`/api/v3/albums/artist/${artistId}`)
    }
    return this.request<PagedResult<Album>>('/api/v3/albums?page=1&pageSize=50')
  }

  async getTracks(): Promise<PagedResult<Track>>
  async getTracks(albumId: number): Promise<Track[]>
  async getTracks(albumId?: number): Promise<Track[] | PagedResult<Track>> {
    if (albumId) {
      return this.request<Track[]>(`/api/v3/tracks/album/${albumId}`)
    }
    return this.request<PagedResult<Track>>('/api/v3/tracks?page=1&pageSize=50')
  }

  async getTrack(id: number): Promise<Track> {
    return this.request<Track>(`/api/v3/tracks/${id}`)
  }

  getStreamUrl(trackId: number): string {
    return `${this.baseUrl}/api/v3/stream/${trackId}`
  }

  getCoverArtUrl(trackId: number, size?: number): string {
    const params = size ? `?width=${size}&height=${size}` : ''
    return `${this.baseUrl}/api/v3/mediacover/track/${trackId}/poster.jpg${params}`
  }

  // --- Library Facets & Filtering ---

  async getFacets(): Promise<LibraryFacets> {
    return this.request<LibraryFacets>('/api/v3/library/facets')
  }

  async filterLibrary(request: FilterRequest): Promise<FilterResponse<Track>> {
    return this.request<FilterResponse<Track>>('/api/v3/library/filter', {
      method: 'POST',
      body: JSON.stringify(request),
    })
  }

  // --- Authors ---

  async getAuthors(page = 1, pageSize = 50): Promise<PagedResult<Author>> {
    return this.request<PagedResult<Author>>(`/api/v3/authors?page=${page}&pageSize=${pageSize}`)
  }

  async getAuthor(id: number): Promise<Author> {
    return this.request<Author>(`/api/v3/authors/${id}`)
  }

  // --- Audiobooks ---

  async getAudiobooks(page = 1, pageSize = 50): Promise<PagedResult<Audiobook>> {
    return this.request<PagedResult<Audiobook>>(`/api/v3/audiobooks?page=${page}&pageSize=${pageSize}`)
  }

  async getAudiobook(id: number): Promise<Audiobook> {
    return this.request<Audiobook>(`/api/v3/audiobooks/${id}`)
  }

  async getAudiobooksByAuthor(authorId: number): Promise<Audiobook[]> {
    return this.request<Audiobook[]>(`/api/v3/audiobooks/author/${authorId}`)
  }

  async getAudiobooksBySeries(seriesId: number): Promise<Audiobook[]> {
    return this.request<Audiobook[]>(`/api/v3/audiobooks/series/${seriesId}`)
  }

  getAudiobookCoverUrl(audiobookId: number, size?: number): string {
    const params = size ? `?width=${size}&height=${size}` : ''
    return `${this.baseUrl}/api/v3/mediacover/${audiobookId}/poster${params}`
  }

  // --- Chapters ---

  async getChapters(mediaFileId: number): Promise<Chapter[]> {
    return this.request<Chapter[]>(`/api/v3/chapters/${mediaFileId}`)
  }

  // --- Progress ---

  async getProgress(mediaItemId: number, userId = 'default'): Promise<MediaProgress> {
    return this.request<MediaProgress>(`/api/v3/progress/${mediaItemId}?userId=${userId}`)
  }

  async updateProgress(mediaItemId: number, positionMs: number, totalDurationMs: number, isComplete = false): Promise<MediaProgress> {
    return this.request<MediaProgress>('/api/v3/progress', {
      method: 'POST',
      body: JSON.stringify({ mediaItemId, positionMs, totalDurationMs, isComplete }),
    })
  }

  // --- Continue Listening ---

  async getContinueListening(limit = 20): Promise<ContinueItem[]> {
    return this.request<ContinueItem[]>(`/api/v3/continue?limit=${limit}`)
  }

  // --- Search ---

  async search(query: string, limit = 50): Promise<SearchResult[]> {
    return this.request<SearchResult[]>(`/api/v3/search?q=${encodeURIComponent(query)}&limit=${limit}`)
  }

  // --- Favorites ---

  async getFavorites(page = 1, pageSize = 50): Promise<PagedResult<Track>> {
    return this.request<PagedResult<Track>>(`/api/v3/favorites?page=${page}&pageSize=${pageSize}`)
  }

  async getFavoriteIds(): Promise<number[]> {
    return this.request<number[]>('/api/v3/favorites/ids')
  }

  async addFavorite(trackId: number): Promise<void> {
    await this.request<void>(`/api/v3/favorites/${trackId}`, { method: 'POST' })
  }

  async removeFavorite(trackId: number): Promise<void> {
    await this.request<void>(`/api/v3/favorites/${trackId}`, { method: 'DELETE' })
  }

  // --- Playlists ---

  async getPlaylists(page = 1, pageSize = 50): Promise<PagedResult<Playlist>> {
    return this.request<PagedResult<Playlist>>(`/api/v3/playlists?page=${page}&pageSize=${pageSize}`)
  }

  async getPlaylist(id: number): Promise<Playlist> {
    return this.request<Playlist>(`/api/v3/playlists/${id}`)
  }

  async createPlaylist(data: { name: string; description?: string }): Promise<Playlist> {
    return this.request<Playlist>('/api/v3/playlists', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  }

  async updatePlaylist(id: number, data: { name?: string; description?: string }): Promise<Playlist> {
    return this.request<Playlist>(`/api/v3/playlists/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    })
  }

  async deletePlaylist(id: number): Promise<void> {
    await this.request<void>(`/api/v3/playlists/${id}`, { method: 'DELETE' })
  }

  async getPlaylistTracks(playlistId: number): Promise<Track[]> {
    return this.request<Track[]>(`/api/v3/playlists/${playlistId}/tracks`)
  }

  async addTrackToPlaylist(playlistId: number, trackId: number): Promise<void> {
    await this.request<void>(`/api/v3/playlists/${playlistId}/tracks`, {
      method: 'POST',
      body: JSON.stringify({ trackId }),
    })
  }

  async removeTrackFromPlaylist(playlistId: number, trackId: number): Promise<void> {
    await this.request<void>(`/api/v3/playlists/${playlistId}/tracks/${trackId}`, {
      method: 'DELETE',
    })
  }

  async reorderPlaylistTracks(playlistId: number, trackIds: number[]): Promise<void> {
    await this.request<void>(`/api/v3/playlists/${playlistId}/tracks/reorder`, {
      method: 'PUT',
      body: JSON.stringify({ trackIds }),
    })
  }

  // --- Scrobbling ---

  async scrobble(entry: Omit<PendingScrobble, 'attempts'>): Promise<void> {
    await this.request<void>('/api/v1/scrobble', {
      method: 'POST',
      body: JSON.stringify(entry),
    })
  }

  // --- Sessions ---

  async getSessions(): Promise<PlaybackSession[]> {
    return this.request<PlaybackSession[]>('/api/v3/sessions')
  }

  async getSession(sessionId: string): Promise<PlaybackSession> {
    return this.request<PlaybackSession>(`/api/v3/sessions/${sessionId}`)
  }

  async getMediaSessions(mediaItemId: number): Promise<PlaybackSession[]> {
    return this.request<PlaybackSession[]>(`/api/v3/sessions/media/${mediaItemId}`)
  }

  async createSession(session: Omit<PlaybackSession, 'id'>): Promise<PlaybackSession> {
    return this.request<PlaybackSession>('/api/v3/sessions', {
      method: 'POST',
      body: JSON.stringify(session),
    })
  }

  async updateSession(sessionId: string, data: Partial<PlaybackSession>): Promise<PlaybackSession> {
    return this.request<PlaybackSession>(`/api/v3/sessions/${sessionId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    })
  }

  async deleteSession(sessionId: string): Promise<void> {
    await this.request<void>(`/api/v3/sessions/${sessionId}`, { method: 'DELETE' })
  }

  // --- History ---

  async getHistory(page = 1, pageSize = 50): Promise<PagedHistory> {
    return this.request<PagedHistory>(`/api/v3/history?page=${page}&pageSize=${pageSize}`)
  }

  async getHistorySince(date: string): Promise<HistoryEntry[]> {
    return this.request<HistoryEntry[]>(`/api/v3/history/since?date=${encodeURIComponent(date)}`)
  }

  async getMediaItemHistory(mediaItemId: number): Promise<HistoryEntry[]> {
    return this.request<HistoryEntry[]>(`/api/v3/history/mediaitem/${mediaItemId}`)
  }

  async addHistoryEntry(entry: Omit<HistoryEntry, 'id'>): Promise<HistoryEntry> {
    return this.request<HistoryEntry>('/api/v3/history', {
      method: 'POST',
      body: JSON.stringify(entry),
    })
  }

  async deleteHistoryEntry(id: number): Promise<void> {
    await this.request<void>(`/api/v3/history/${id}`, { method: 'DELETE' })
  }

  // --- Podcasts ---

  async getPodcasts(page = 1, pageSize = 50): Promise<PagedResult<PodcastShow>> {
    return this.request<PagedResult<PodcastShow>>(`/api/v3/podcasts?page=${page}&pageSize=${pageSize}`)
  }

  async getPodcast(id: number): Promise<PodcastShow> {
    return this.request<PodcastShow>(`/api/v3/podcasts/${id}`)
  }

  async addPodcast(podcast: Omit<PodcastShow, 'id' | 'added'>): Promise<PodcastShow> {
    return this.request<PodcastShow>('/api/v3/podcasts', {
      method: 'POST',
      body: JSON.stringify(podcast),
    })
  }

  async updatePodcast(id: number, podcast: Partial<PodcastShow>): Promise<PodcastShow> {
    return this.request<PodcastShow>(`/api/v3/podcasts/${id}`, {
      method: 'PUT',
      body: JSON.stringify(podcast),
    })
  }

  async deletePodcast(id: number): Promise<void> {
    await this.request<void>(`/api/v3/podcasts/${id}`, { method: 'DELETE' })
  }

  async getPodcastEpisodes(podcastId: number): Promise<PodcastEpisode[]> {
    return this.request<PodcastEpisode[]>(`/api/v3/podcasts/${podcastId}/episodes`)
  }

  async getPodcastEpisode(episodeId: number): Promise<PodcastEpisode> {
    return this.request<PodcastEpisode>(`/api/v3/podcasts/episodes/${episodeId}`)
  }
}

export const apiClient = new ApiClient()

// Helper functions for components
export function getStreamUrl(trackId: number): string {
  return apiClient.getStreamUrl(trackId)
}

export function getCoverArtUrl(trackId: number, size?: number): string {
  return apiClient.getCoverArtUrl(trackId, size)
}
