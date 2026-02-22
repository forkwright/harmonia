// Mock data for sessions, history, podcasts, playlists, and auth
import type {
  User, PlaybackSession, HistoryEntry, PodcastShow, PodcastEpisode, Playlist,
} from '../types'

export const mockUser: User = {
  id: 1,
  username: 'admin',
  displayName: 'Admin User',
  email: 'admin@localhost',
  role: 'admin',
  authenticationMethod: 'forms',
  isActive: true,
  createdAt: '2025-01-01T00:00:00Z',
  lastLoginAt: '2026-02-21T12:00:00Z',
}

export const mockSessions: PlaybackSession[] = [
  {
    id: 1,
    sessionId: 'sess-001',
    mediaItemId: 1,
    userId: 'admin',
    deviceName: 'Web Browser',
    deviceType: 'web',
    startedAt: '2026-02-21T10:00:00Z',
    endedAt: '2026-02-21T10:45:00Z',
    startPositionMs: 0,
    endPositionMs: 270000,
    durationMs: 2700000,
    isActive: false,
  },
  {
    id: 2,
    sessionId: 'sess-002',
    mediaItemId: 3,
    userId: 'admin',
    deviceName: 'Pixel 8',
    deviceType: 'android',
    startedAt: '2026-02-21T14:00:00Z',
    startPositionMs: 0,
    durationMs: 1800000,
    isActive: true,
  },
  {
    id: 3,
    sessionId: 'sess-003',
    mediaItemId: 2,
    userId: 'admin',
    deviceName: 'Web Browser',
    deviceType: 'web',
    startedAt: '2025-02-21T09:00:00Z',
    endedAt: '2025-02-21T10:30:00Z',
    startPositionMs: 0,
    endPositionMs: 540000,
    durationMs: 5400000,
    isActive: false,
  },
]

export const mockHistoryEntries: HistoryEntry[] = [
  {
    id: 1,
    mediaItemId: 1,
    mediaType: 3,
    sourceTitle: 'Radiohead - OK Computer (FLAC)',
    quality: { quality: { id: 7, name: 'FLAC' }, revision: { version: 1, real: 0 } },
    date: '2026-02-20T15:30:00Z',
    eventType: 3,
    data: { importedPath: '/music/Radiohead/OK Computer' },
  },
  {
    id: 2,
    mediaItemId: 2,
    mediaType: 3,
    sourceTitle: 'Pink Floyd - The Dark Side of the Moon (FLAC)',
    quality: { quality: { id: 7, name: 'FLAC' }, revision: { version: 1, real: 0 } },
    date: '2026-02-19T10:00:00Z',
    eventType: 3,
    data: { importedPath: '/music/Pink Floyd/The Dark Side of the Moon' },
  },
  {
    id: 3,
    mediaItemId: 5,
    mediaType: 3,
    sourceTitle: 'Tool - Lateralus (FLAC)',
    quality: { quality: { id: 7, name: 'FLAC' }, revision: { version: 1, real: 0 } },
    date: '2026-02-18T08:00:00Z',
    eventType: 1,
    data: { downloadClient: 'Transmission' },
  },
]

export const mockPlaylists: Playlist[] = [
  {
    id: 1,
    name: 'Late Night Prog',
    description: 'Progressive rock for quiet hours',
    trackCount: 8,
    totalDuration: 2400000,
    createdAt: '2026-01-15T00:00:00Z',
    updatedAt: '2026-02-20T00:00:00Z',
  },
  {
    id: 2,
    name: 'Jazz Essentials',
    trackCount: 5,
    totalDuration: 1800000,
    createdAt: '2026-02-01T00:00:00Z',
    updatedAt: '2026-02-18T00:00:00Z',
  },
]

// Mutable — track assignments per playlist
export const mockPlaylistTracks: Record<number, number[]> = {
  1: [1, 3, 5, 7],
  2: [2, 4],
}

let nextPlaylistId = 3

export function createMockPlaylist(name: string, description?: string): Playlist {
  const playlist: Playlist = {
    id: nextPlaylistId++,
    name,
    description,
    trackCount: 0,
    totalDuration: 0,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  }
  mockPlaylists.push(playlist)
  mockPlaylistTracks[playlist.id] = []
  return playlist
}

export const mockPodcastShows: PodcastShow[] = [
  {
    id: 1,
    title: 'Darknet Diaries',
    description: 'True stories from the dark side of the Internet.',
    author: 'Jack Rhysider',
    feedUrl: 'https://feeds.megaphone.fm/darknetdiaries',
    imageUrl: 'https://picsum.photos/400',
    categories: 'Technology',
    language: 'en',
    website: 'https://darknetdiaries.com',
    episodeCount: 150,
    latestEpisodeDate: '2026-02-18T00:00:00Z',
    monitored: true,
    monitorNewEpisodes: true,
    qualityProfileId: 1,
    added: '2026-01-01T00:00:00Z',
  },
  {
    id: 2,
    title: 'Hardcore History',
    description: 'In-depth looks at past civilizations, historical events, and figures.',
    author: 'Dan Carlin',
    feedUrl: 'https://feeds.feedburner.com/dancarlin',
    imageUrl: 'https://picsum.photos/401',
    categories: 'History',
    language: 'en',
    website: 'https://www.dancarlin.com',
    episodeCount: 75,
    latestEpisodeDate: '2026-01-15T00:00:00Z',
    monitored: true,
    monitorNewEpisodes: true,
    qualityProfileId: 1,
    added: '2026-01-05T00:00:00Z',
  },
]

export const mockPodcastEpisodes: PodcastEpisode[] = [
  {
    id: 1,
    podcastShowId: 1,
    title: 'EP 150: The Ransomware Cartel',
    description: 'A deep dive into one of the most sophisticated ransomware groups.',
    episodeNumber: 150,
    publishDate: '2026-02-18T00:00:00Z',
    duration: 4200,
    enclosureUrl: 'https://example.com/ep150.mp3',
    enclosureType: 'audio/mpeg',
    explicit: false,
    monitored: true,
    added: '2026-02-18T01:00:00Z',
  },
  {
    id: 2,
    podcastShowId: 1,
    title: 'EP 149: The Hospital Hack',
    description: 'When hackers target healthcare systems.',
    episodeNumber: 149,
    publishDate: '2026-02-04T00:00:00Z',
    duration: 3600,
    enclosureUrl: 'https://example.com/ep149.mp3',
    enclosureType: 'audio/mpeg',
    explicit: false,
    monitored: true,
    added: '2026-02-04T01:00:00Z',
  },
  {
    id: 3,
    podcastShowId: 2,
    title: 'Supernova in the East VI',
    description: 'The final chapter of the Pacific War series.',
    episodeNumber: 6,
    seasonNumber: 1,
    publishDate: '2026-01-15T00:00:00Z',
    duration: 14400,
    enclosureUrl: 'https://example.com/hh-sne6.mp3',
    enclosureType: 'audio/mpeg',
    explicit: false,
    monitored: true,
    added: '2026-01-15T01:00:00Z',
  },
]
