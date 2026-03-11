// Manual playlists (Sylloges) — backend-persisted, music tracks only
import { create } from 'zustand'
import { apiClient } from '../api/client'
import type { Playlist, Track } from '../types'

interface PlaylistState {
  playlists: Playlist[]
  loading: boolean
  error: string | null

  loadPlaylists: () => Promise<void>
  createPlaylist: (name: string, description?: string) => Promise<Playlist>
  updatePlaylist: (id: number, data: { name?: string; description?: string }) => Promise<void>
  deletePlaylist: (id: number) => Promise<void>

  getPlaylistTracks: (playlistId: number) => Promise<Track[]>
  addTrackToPlaylist: (playlistId: number, trackId: number) => Promise<void>
  removeTrackFromPlaylist: (playlistId: number, trackId: number) => Promise<void>
  reorderPlaylistTracks: (playlistId: number, trackIds: number[]) => Promise<void>
}

export const usePlaylistStore = create<PlaylistState>((set, get) => ({
  playlists: [],
  loading: false,
  error: null,

  loadPlaylists: async () => {
    set({ loading: true, error: null })
    try {
      const result = await apiClient.getPlaylists()
      set({ playlists: result.items })
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to load playlists' })
    } finally {
      set({ loading: false })
    }
  },

  createPlaylist: async (name, description) => {
    const playlist = await apiClient.createPlaylist({ name, description })
    set({ playlists: [...get().playlists, playlist] })
    return playlist
  },

  updatePlaylist: async (id, data) => {
    const updated = await apiClient.updatePlaylist(id, data)
    set({
      playlists: get().playlists.map((p) => (p.id === id ? updated : p)),
    })
  },

  deletePlaylist: async (id) => {
    await apiClient.deletePlaylist(id)
    set({ playlists: get().playlists.filter((p) => p.id !== id) })
  },

  getPlaylistTracks: (playlistId) => apiClient.getPlaylistTracks(playlistId),

  addTrackToPlaylist: async (playlistId, trackId) => {
    await apiClient.addTrackToPlaylist(playlistId, trackId)
    // Update track count optimistically
    set({
      playlists: get().playlists.map((p) =>
        p.id === playlistId ? { ...p, trackCount: p.trackCount + 1 } : p
      ),
    })
  },

  removeTrackFromPlaylist: async (playlistId, trackId) => {
    await apiClient.removeTrackFromPlaylist(playlistId, trackId)
    set({
      playlists: get().playlists.map((p) =>
        p.id === playlistId ? { ...p, trackCount: Math.max(0, p.trackCount - 1) } : p
      ),
    })
  },

  reorderPlaylistTracks: (playlistId, trackIds) =>
    apiClient.reorderPlaylistTracks(playlistId, trackIds),
}))
