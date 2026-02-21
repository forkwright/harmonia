// API types matching Mouseion backend

// --- Music Types ---

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

// --- Audiobook Types ---

export interface Author {
  id: number
  name: string
  sortName?: string
  description?: string
  foreignAuthorId?: string
  monitored: boolean
  path?: string
  rootFolderPath?: string
  qualityProfileId: number
  added: string
  tags?: number[]
}

export interface AudiobookMetadata {
  description?: string
  foreignAudiobookId?: string
  audnexusId?: string
  audibleId?: string
  isbn?: string
  isbn13?: string
  asin?: string
  releaseDate?: string
  publisher?: string
  language?: string
  genres: string[]
  narrator?: string
  narrators: string[]
  durationMinutes?: number
  isAbridged: boolean
  seriesPosition?: number
  bookId?: number
}

export interface Audiobook {
  id: number
  title: string
  year: number
  monitored: boolean
  qualityProfileId: number
  added: string
  authorId?: number
  bookSeriesId?: number
  tags?: number[]
  metadata: AudiobookMetadata
}

export interface Chapter {
  title: string
  startTimeMs: number
  endTimeMs: number
  index: number
}

// --- Progress Types ---

export interface MediaProgress {
  id: number
  mediaItemId: number
  userId: string
  positionMs: number
  totalDurationMs: number
  percentComplete: number
  lastPlayedAt: string
  isComplete: boolean
  createdAt: string
  updatedAt: string
}

export interface ContinueItem {
  mediaItemId: number
  title: string
  mediaType: string
  positionMs: number
  totalDurationMs: number
  percentComplete: number
  lastPlayedAt: string
  mediaFileId?: number
  coverUrl: string
}

export interface PlaybackSession {
  id: number
  sessionId: string
  mediaItemId: number
  userId: string
  deviceName: string
  deviceType: string
  startedAt: string
  endedAt?: string
  startPositionMs: number
  endPositionMs?: number
  durationMs: number
  isActive: boolean
}

// --- Search Types ---

export interface SearchResult {
  trackId: number
  title: string
  artist?: string
  album?: string
  trackNumber: number
  discNumber: number
  durationSeconds?: number
  genre?: string
  bitDepth?: number
  dynamicRange?: number
  lossless: boolean
  relevanceScore: number
}

// --- Paged Response ---

export interface PagedResult<T> {
  items: T[]
  page: number
  pageSize: number
  totalCount: number
}

// --- Auth ---

export interface AuthResponse {
  token: string
  expiresIn: number
}

export interface ApiError {
  message: string
  code?: string
}

// --- Playback State ---

export interface PlaybackState {
  currentTrack: Track | null
  isPlaying: boolean
  position: number
  duration: number
  volume: number
}
