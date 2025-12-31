// API types matching Mouseion backend
export interface Track {
  id: number
  title: string
  artist: string
  album: string
  duration: number
  fileSize: number
  format: string
  bitrate: number
  sampleRate: number
  bitDepth: number
  channels: number
  coverArtUrl?: string
}

export interface Album {
  id: number
  title: string
  artist: string
  year?: number
  trackCount: number
  duration: number
  coverArtUrl?: string
}

export interface Artist {
  id: number
  name: string
  albumCount: number
  trackCount: number
}

export interface AuthResponse {
  token: string
  expiresIn: number
}

export interface ApiError {
  message: string
  code?: string
}

export interface PlaybackState {
  currentTrack: Track | null
  isPlaying: boolean
  position: number
  duration: number
  volume: number
}
