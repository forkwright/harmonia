// Audio playback state store
import { create } from 'zustand'
import type { Track } from '../types'

interface PlayerState {
  currentTrack: Track | null
  isPlaying: boolean
  position: number
  duration: number
  volume: number
  queue: Track[]

  setCurrentTrack: (track: Track | null) => void
  setIsPlaying: (playing: boolean) => void
  setPosition: (position: number) => void
  setDuration: (duration: number) => void
  setVolume: (volume: number) => void
  addToQueue: (track: Track) => void
  clearQueue: () => void
}

export const usePlayerStore = create<PlayerState>((set) => ({
  currentTrack: null,
  isPlaying: false,
  position: 0,
  duration: 0,
  volume: 1.0,
  queue: [],

  setCurrentTrack: (track) => set({ currentTrack: track, position: 0 }),
  setIsPlaying: (playing) => set({ isPlaying: playing }),
  setPosition: (position) => set({ position }),
  setDuration: (duration) => set({ duration }),
  setVolume: (volume) => set({ volume: Math.max(0, Math.min(1, volume)) }),
  addToQueue: (track) => set((state) => ({ queue: [...state.queue, track] })),
  clearQueue: () => set({ queue: [] }),
}))
