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

export interface Bookmark {
  id: string
  audiobookId: number
  positionMs: number
  chapterTitle: string
  note: string
  createdAt: string
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

// --- History Types ---

export interface HistoryEntry {
  id: number
  mediaItemId: number
  mediaType: number
  sourceTitle: string
  quality: QualityModel
  date: string
  eventType: number
  data: Record<string, unknown>
  downloadId?: string
}

export interface QualityModel {
  quality: { id: number; name: string }
  revision: { version: number; real: number }
}

export interface PagedHistory {
  page: number
  pageSize: number
  sortKey?: string
  sortDirection?: string
  totalRecords: number
  records: HistoryEntry[]
}

// --- Podcast Types ---

export interface PodcastShow {
  id: number
  title: string
  sortTitle?: string
  description?: string
  foreignPodcastId?: string
  itunesId?: string
  author?: string
  feedUrl: string
  imageUrl?: string
  categories?: string
  language?: string
  website?: string
  episodeCount?: number
  latestEpisodeDate?: string
  monitored: boolean
  monitorNewEpisodes: boolean
  path?: string
  rootFolderPath?: string
  qualityProfileId: number
  tags?: string
  added: string
  lastSearchTime?: string
}

export interface PodcastEpisode {
  id: number
  podcastShowId: number
  title: string
  description?: string
  episodeGuid?: string
  episodeNumber?: number
  seasonNumber?: number
  publishDate?: string
  duration?: number
  enclosureUrl?: string
  enclosureLength?: number
  enclosureType?: string
  imageUrl?: string
  explicit: boolean
  monitored: boolean
  added: string
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

export type SearchResultType = 'track' | 'audiobook' | 'podcast'

export interface UnifiedSearchResult {
  id: number
  type: SearchResultType
  title: string
  subtitle: string
  coverUrl?: string
}

// --- EQ Types ---

export type FilterType = 'peaking' | 'low_shelf' | 'high_shelf' | 'low_pass' | 'high_pass'

export interface ParametricBand {
  type: FilterType
  frequency: number
  gain: number
  q: number
}

export interface HeadphoneProfile {
  manufacturer: string
  model: string
  parametricEq: ParametricBand[]
}

// --- Paged Response ---

export interface PagedResult<T> {
  items: T[]
  page: number
  pageSize: number
  totalCount: number
}

// --- Auth ---

export interface User {
  id: number
  username: string
  displayName: string
  email: string
  role: string
  authenticationMethod: string
  isActive: boolean
  createdAt: string
  lastLoginAt?: string
}

export interface AuthResponse {
  accessToken: string
  refreshToken: string
  user: User
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

// --- Scrobble Types ---

export interface PendingScrobble {
  artist: string
  track: string
  album: string
  timestamp: number
  duration: number
  attempts: number
}
