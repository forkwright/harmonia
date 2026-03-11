// Playlist (Sylloges) store tests
import { describe, it, expect, beforeEach, vi } from 'vitest'
import type { Playlist, Track, PagedResult } from '../types'

const mockPlaylist: Playlist = {
  id: 1, name: 'Test Playlist', trackCount: 3, totalDuration: 600000,
  createdAt: '2026-01-01T00:00:00Z', updatedAt: '2026-02-01T00:00:00Z',
}
const mockPlaylist2: Playlist = {
  id: 2, name: 'Jazz', trackCount: 2, totalDuration: 400000,
  createdAt: '2026-01-10T00:00:00Z', updatedAt: '2026-02-10T00:00:00Z',
}
const mockTracks: Track[] = [
  { id: 1, title: 'T1', artist: 'A', album: 'Al', duration: 200, fileSize: 5e6, format: 'FLAC', bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2 },
  { id: 2, title: 'T2', artist: 'A', album: 'Al', duration: 200, fileSize: 5e6, format: 'FLAC', bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2 },
]

const mockGetPlaylists = vi.fn().mockResolvedValue({ items: [mockPlaylist, mockPlaylist2], page: 1, pageSize: 50, totalCount: 2 } as PagedResult<Playlist>)
const mockCreatePlaylist = vi.fn().mockResolvedValue({ ...mockPlaylist, id: 3, name: 'New' })
const mockUpdatePlaylist = vi.fn().mockResolvedValue({ ...mockPlaylist, name: 'Renamed' })
const mockDeletePlaylist = vi.fn().mockResolvedValue(undefined)
const mockGetPlaylistTracks = vi.fn().mockResolvedValue(mockTracks)
const mockAddTrackToPlaylist = vi.fn().mockResolvedValue(undefined)
const mockRemoveTrackFromPlaylist = vi.fn().mockResolvedValue(undefined)
const mockReorderPlaylistTracks = vi.fn().mockResolvedValue(undefined)

vi.mock('../api/client', () => ({
  apiClient: {
    getPlaylists: (...args: unknown[]) => mockGetPlaylists(...args),
    createPlaylist: (...args: unknown[]) => mockCreatePlaylist(...args),
    updatePlaylist: (...args: unknown[]) => mockUpdatePlaylist(...args),
    deletePlaylist: (...args: unknown[]) => mockDeletePlaylist(...args),
    getPlaylistTracks: (...args: unknown[]) => mockGetPlaylistTracks(...args),
    addTrackToPlaylist: (...args: unknown[]) => mockAddTrackToPlaylist(...args),
    removeTrackFromPlaylist: (...args: unknown[]) => mockRemoveTrackFromPlaylist(...args),
    reorderPlaylistTracks: (...args: unknown[]) => mockReorderPlaylistTracks(...args),
  },
}))

import { usePlaylistStore } from './playlistStore'

describe('playlistStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    usePlaylistStore.setState({ playlists: [], loading: false, error: null })
  })

  describe('loadPlaylists', () => {
    it('loads playlists from API', async () => {
      await usePlaylistStore.getState().loadPlaylists()

      expect(usePlaylistStore.getState().playlists).toHaveLength(2)
      expect(usePlaylistStore.getState().playlists[0].name).toBe('Test Playlist')
      expect(usePlaylistStore.getState().loading).toBe(false)
    })

    it('sets error on failure', async () => {
      mockGetPlaylists.mockRejectedValueOnce(new Error('Network fail'))
      await usePlaylistStore.getState().loadPlaylists()

      expect(usePlaylistStore.getState().error).toBe('Network fail')
      expect(usePlaylistStore.getState().loading).toBe(false)
    })
  })

  describe('createPlaylist', () => {
    it('creates and adds to store', async () => {
      const result = await usePlaylistStore.getState().createPlaylist('New', 'desc')

      expect(mockCreatePlaylist).toHaveBeenCalledWith({ name: 'New', description: 'desc' })
      expect(result.name).toBe('New')
      expect(usePlaylistStore.getState().playlists).toHaveLength(1)
    })
  })

  describe('updatePlaylist', () => {
    it('updates playlist in store', async () => {
      usePlaylistStore.setState({ playlists: [mockPlaylist] })
      await usePlaylistStore.getState().updatePlaylist(1, { name: 'Renamed' })

      expect(usePlaylistStore.getState().playlists[0].name).toBe('Renamed')
    })
  })

  describe('deletePlaylist', () => {
    it('removes playlist from store', async () => {
      usePlaylistStore.setState({ playlists: [mockPlaylist, mockPlaylist2] })
      await usePlaylistStore.getState().deletePlaylist(1)

      expect(usePlaylistStore.getState().playlists).toHaveLength(1)
      expect(usePlaylistStore.getState().playlists[0].id).toBe(2)
    })
  })

  describe('track management', () => {
    it('addTrackToPlaylist increments trackCount', async () => {
      usePlaylistStore.setState({ playlists: [{ ...mockPlaylist, trackCount: 3 }] })
      await usePlaylistStore.getState().addTrackToPlaylist(1, 99)

      expect(mockAddTrackToPlaylist).toHaveBeenCalledWith(1, 99)
      expect(usePlaylistStore.getState().playlists[0].trackCount).toBe(4)
    })

    it('removeTrackFromPlaylist decrements trackCount', async () => {
      usePlaylistStore.setState({ playlists: [{ ...mockPlaylist, trackCount: 3 }] })
      await usePlaylistStore.getState().removeTrackFromPlaylist(1, 99)

      expect(usePlaylistStore.getState().playlists[0].trackCount).toBe(2)
    })

    it('removeTrackFromPlaylist does not go below 0', async () => {
      usePlaylistStore.setState({ playlists: [{ ...mockPlaylist, trackCount: 0 }] })
      await usePlaylistStore.getState().removeTrackFromPlaylist(1, 99)

      expect(usePlaylistStore.getState().playlists[0].trackCount).toBe(0)
    })

    it('getPlaylistTracks delegates to API', async () => {
      const tracks = await usePlaylistStore.getState().getPlaylistTracks(1)
      expect(mockGetPlaylistTracks).toHaveBeenCalledWith(1)
      expect(tracks).toHaveLength(2)
    })

    it('reorderPlaylistTracks delegates to API', async () => {
      await usePlaylistStore.getState().reorderPlaylistTracks(1, [2, 1])
      expect(mockReorderPlaylistTracks).toHaveBeenCalledWith(1, [2, 1])
    })
  })
})
