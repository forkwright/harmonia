// Audio playback state store
import { create } from 'zustand'
import type { Track } from '../types'
import { syncService } from '../services/syncService'
import { sessionManager } from '../services/sessionManager'

interface PlayerState {
  currentTrack: Track | null
  mediaItemId: number | null
  isPlaying: boolean
  position: number
  duration: number
  volume: number
  playbackSpeed: number
  queue: Track[]
  syncCleanup: (() => void) | null

  setCurrentTrack: (track: Track | null) => void
  startPlayback: (track: Track, mediaItemId: number) => void
  stopPlayback: () => void
  setIsPlaying: (playing: boolean) => void
  setPosition: (position: number) => void
  setDuration: (duration: number) => void
  setVolume: (volume: number) => void
  setPlaybackSpeed: (speed: number) => void
  addToQueue: (track: Track) => void
  setQueue: (queue: Track[]) => void
  removeFromQueue: (index: number) => void
  clearQueue: () => void
}

export const usePlayerStore = create<PlayerState>((set, get) => ({
  currentTrack: null,
  mediaItemId: null,
  isPlaying: false,
  position: 0,
  duration: 0,
  volume: 1,
  playbackSpeed: 1,
  queue: [],
  syncCleanup: null,

  setCurrentTrack: (track) => set({ currentTrack: track, position: 0 }),

  startPlayback: (track, mediaItemId) => {
    const prev = get()
    prev.syncCleanup?.()
    if (prev.mediaItemId) {
      void syncService.reportProgress(prev.mediaItemId, prev.position, prev.duration)
      void sessionManager.endSession(prev.position)
    }

    set({ currentTrack: track, mediaItemId, position: 0, isPlaying: true })

    const cleanup = syncService.startAutoSync(() => {
      const s = get()
      if (!s.mediaItemId || !s.isPlaying) return null
      return {
        mediaItemId: s.mediaItemId,
        positionMs: Math.round(s.position),
        totalDurationMs: Math.round(s.duration),
      }
    })
    set({ syncCleanup: cleanup })

    void sessionManager.startSession({
      mediaItemId,
      mediaType: 'music',
      positionMs: 0,
      totalDurationMs: Math.round(track.duration),
    })
  },

  stopPlayback: () => {
    const { syncCleanup, position, duration, mediaItemId } = get()
    syncCleanup?.()
    if (mediaItemId) {
      void syncService.reportProgress(mediaItemId, Math.round(position), Math.round(duration))
      void sessionManager.endSession(Math.round(position))
    }
    set({ isPlaying: false, syncCleanup: null, mediaItemId: null })
  },

  setIsPlaying: (playing) => set({ isPlaying: playing }),
  setPosition: (position) => set({ position }),
  setDuration: (duration) => set({ duration }),
  setVolume: (volume) => set({ volume: Math.max(0, Math.min(1, volume)) }),
  setPlaybackSpeed: (speed) => set({ playbackSpeed: Math.max(0.5, Math.min(2, speed)) }),
  addToQueue: (track) => set((state) => ({ queue: [...state.queue, track] })),
  setQueue: (queue) => set({ queue }),
  removeFromQueue: (index) => set((state) => ({
    queue: state.queue.filter((_, i) => i !== index)
  })),
  clearQueue: () => set({ queue: [] }),
}))
