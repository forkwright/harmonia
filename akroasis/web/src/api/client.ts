// Mouseion API client
import type {
  Track, Album, Artist, AuthResponse, ApiError,
  Audiobook, Author, Chapter, MediaProgress, ContinueItem,
  PagedResult, SearchResult, PendingScrobble,
} from '../types'

class ApiClient {
  private baseUrl: string
  private apiKey: string | null = null

  constructor(baseUrl: string = '') {
    const storedUrl = localStorage.getItem('serverUrl')
    const defaultUrl = import.meta.env.MODE === 'development' ? 'http://localhost:5000' : ''
    this.baseUrl = (baseUrl || storedUrl || defaultUrl).replace(/\/$/, '')

    const stored = localStorage.getItem('apiKey')
    if (stored) {
      this.apiKey = stored
    }
  }

  setServerUrl(url: string) {
    this.baseUrl = url.replace(/\/$/, '')
    localStorage.setItem('serverUrl', this.baseUrl)
  }

  setApiKey(key: string) {
    this.apiKey = key
    localStorage.setItem('apiKey', key)
  }

  clearAuth() {
    this.apiKey = null
    localStorage.removeItem('apiKey')
  }

  private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
      ...(this.apiKey && { 'X-Api-Key': this.apiKey }),
      ...options.headers,
    }

    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      headers,
    })

    if (!response.ok) {
      const error: ApiError = await response.json().catch(() => ({
        message: `HTTP ${response.status}: ${response.statusText}`,
      }))
      throw new Error(error.message)
    }

    return response.json()
  }

  // --- Auth ---

  async login(username: string, password: string): Promise<AuthResponse> {
    return this.request<AuthResponse>('/api/v3/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    })
  }

  // --- Music ---

  async getArtists(): Promise<Artist[]> {
    return this.request<Artist[]>('/api/v3/artists')
  }

  async getAlbums(artistId?: number): Promise<Album[]> {
    const endpoint = artistId
      ? `/api/v3/artists/${artistId}/albums`
      : '/api/v3/albums'
    return this.request<Album[]>(endpoint)
  }

  async getTracks(albumId?: number): Promise<Track[]> {
    const endpoint = albumId
      ? `/api/v3/albums/${albumId}/tracks`
      : '/api/v3/tracks'
    return this.request<Track[]>(endpoint)
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

  // --- Scrobbling ---

  async scrobble(entry: Omit<PendingScrobble, 'attempts'>): Promise<void> {
    await this.request<void>('/api/v1/scrobble', {
      method: 'POST',
      body: JSON.stringify(entry),
    })
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
