// Audio playback state store
import { create } from 'zustand'
import type { Track } from '../types'
import { syncService } from '../services/syncService'
import { sessionManager } from '../services/sessionManager'
import { loadJson } from '../utils/storage'

export type RepeatMode = 'off' | 'all' | 'one' | 'shuffle-repeat'

function fisherYatesShuffle<T>(array: T[]): T[] {
  const shuffled = [...array]
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]]
  }
  return shuffled
}

interface PlayerState {
  currentTrack: Track | null
  mediaItemId: number | null
  isPlaying: boolean
  position: number
  duration: number
  volume: number
  playbackSpeed: number
  queue: Track[]
  repeatMode: RepeatMode
  originalQueue: Track[]
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
  cycleRepeatMode: () => void
  setRepeatMode: (mode: RepeatMode) => void
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
  repeatMode: loadJson<RepeatMode>('akroasis_repeat_mode', 'off'),
  originalQueue: [],
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
  clearQueue: () => set({ queue: [], originalQueue: [] }),

  cycleRepeatMode: () => {
    const order: RepeatMode[] = ['off', 'all', 'one', 'shuffle-repeat']
    const current = get().repeatMode
    const nextIndex = (order.indexOf(current) + 1) % order.length
    const next = order[nextIndex]

    if (next === 'shuffle-repeat') {
      const { queue } = get()
      set({ repeatMode: next, originalQueue: [...queue], queue: fisherYatesShuffle(queue) })
    } else if (current === 'shuffle-repeat') {
      const { originalQueue } = get()
      set({ repeatMode: next, queue: originalQueue.length > 0 ? originalQueue : get().queue, originalQueue: [] })
    } else {
      set({ repeatMode: next })
    }
    localStorage.setItem('akroasis_repeat_mode', JSON.stringify(next))
  },

  setRepeatMode: (mode) => {
    const current = get().repeatMode
    if (mode === 'shuffle-repeat' && current !== 'shuffle-repeat') {
      const { queue } = get()
      set({ repeatMode: mode, originalQueue: [...queue], queue: fisherYatesShuffle(queue) })
    } else if (mode !== 'shuffle-repeat' && current === 'shuffle-repeat') {
      const { originalQueue } = get()
      set({ repeatMode: mode, queue: originalQueue.length > 0 ? originalQueue : get().queue, originalQueue: [] })
    } else {
      set({ repeatMode: mode })
    }
    localStorage.setItem('akroasis_repeat_mode', JSON.stringify(mode))
  },
}))
