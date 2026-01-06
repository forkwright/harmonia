// Audio playback state store
import { create } from 'zustand'
import type { Track } from '../types'

interface PlayerState {
  currentTrack: Track | null
  isPlaying: boolean
  position: number
  duration: number
  volume: number
  playbackSpeed: number
  queue: Track[]

  setCurrentTrack: (track: Track | null) => void
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

export const usePlayerStore = create<PlayerState>((set) => ({
  currentTrack: null,
  isPlaying: false,
  position: 0,
  duration: 0,
  volume: 1,
  playbackSpeed: 1,
  queue: [],

  setCurrentTrack: (track) => set({ currentTrack: track, position: 0 }),
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
