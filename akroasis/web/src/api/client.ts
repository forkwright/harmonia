// Mouseion API client
import type { Track, Album, Artist, AuthResponse, ApiError } from '../types'

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

  async login(username: string, password: string): Promise<AuthResponse> {
    return this.request<AuthResponse>('/api/v3/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    })
  }

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
}

export const apiClient = new ApiClient()

// Helper functions for components
export function getStreamUrl(trackId: number): string {
  return apiClient.getStreamUrl(trackId)
}

export function getCoverArtUrl(trackId: number, size?: number): string {
  return apiClient.getCoverArtUrl(trackId, size)
}
