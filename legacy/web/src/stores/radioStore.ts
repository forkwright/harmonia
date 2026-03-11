// Radio mode state — auto-builds queue from Last.fm similar tracks
import { create } from 'zustand'
import type { Track } from '../types'
import { buildRadioTracks, isLastfmConfigured } from '../api/lastfm'
import { usePlayerStore } from './playerStore'

// Minimum queue depth before replenishment triggers
const REPLENISH_THRESHOLD = 2

interface RadioState {
  radioMode: boolean
  radioSeed: Track | null
  loading: boolean
  error: string | null

  startRadio: (track: Track) => Promise<void>
  stopRadio: () => void
  replenishIfNeeded: (currentTrack: Track | null) => Promise<void>
}

export const useRadioStore = create<RadioState>((set, get) => ({
  radioMode: false,
  radioSeed: null,
  loading: false,
  error: null,

  startRadio: async (seed: Track) => {
    if (!isLastfmConfigured()) {
      set({ error: 'Last.fm API key not configured. Set VITE_LASTFM_API_KEY to enable radio.' })
      return
    }

    set({ radioMode: true, radioSeed: seed, loading: true, error: null })

    const { queue, currentTrack, setQueue, setCurrentTrack, setIsPlaying } = usePlayerStore.getState()
    const excludeIds = new Set<number>([seed.id, ...queue.map((t) => t.id)])
    if (currentTrack) excludeIds.add(currentTrack.id)

    try {
      const tracks = await buildRadioTracks(seed, excludeIds)

      if (tracks.length === 0) {
        set({
          loading: false,
          error: 'No matching tracks found in your library for this seed.',
          radioMode: false,
          radioSeed: null,
        })
        return
      }

      // Replace queue with radio tracks; seed plays immediately
      setCurrentTrack(seed)
      setIsPlaying(true)
      setQueue(tracks)
      set({ loading: false, error: null })
    } catch (err) {
      set({
        loading: false,
        radioMode: false,
        radioSeed: null,
        error: err instanceof Error ? err.message : 'Radio failed to start',
      })
    }
  },

  stopRadio: () => {
    set({ radioMode: false, radioSeed: null, loading: false, error: null })
  },

  replenishIfNeeded: async (currentTrack: Track | null) => {
    const { radioMode, radioSeed, loading } = get()
    if (!radioMode || !radioSeed || loading) return

    const { queue } = usePlayerStore.getState()
    const seed = currentTrack ?? radioSeed

    // Only replenish when queue is running low
    if (queue.length > REPLENISH_THRESHOLD) return

    set({ loading: true })

    const excludeIds = new Set<number>([
      radioSeed.id,
      seed.id,
      ...queue.map((t) => t.id),
    ])

    try {
      const newTracks = await buildRadioTracks(seed, excludeIds)
      if (newTracks.length > 0) {
        const { queue: currentQueue, setQueue } = usePlayerStore.getState()
        setQueue([...currentQueue, ...newTracks])
      }
    } catch {
      // Silent — replenishment failure is non-fatal
    } finally {
      set({ loading: false })
    }
  },
}))
