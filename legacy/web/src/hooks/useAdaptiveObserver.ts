// Adaptive experience observer — watches playback and navigation,
// updates the listening profile. Wire into App.tsx once.
import { useEffect, useRef } from 'react'
import { useLocation } from 'react-router-dom'
import { usePlayerStore } from '../stores/playerStore'
import { useListeningProfileStore } from '../stores/listeningProfileStore'

const MIN_PLAY_SECONDS = 30 // Count a play only after 30s

// Map routes to feature names for tracking
const ROUTE_FEATURES: Record<string, string> = {
  '/': 'discovery',
  '/library': 'library',
  '/player': 'player',
  '/queue': 'queue',
  '/playlists': 'playlists',
  '/audiobooks': 'audiobooks',
  '/podcasts': 'podcasts',
  '/settings': 'settings',
}

export function useAdaptiveObserver() {
  const currentTrack = usePlayerStore(s => s.currentTrack)
  const isPlaying = usePlayerStore(s => s.isPlaying)
  const position = usePlayerStore(s => s.position)
  const recordPlay = useListeningProfileStore(s => s.recordPlay)
  const recordFeatureUse = useListeningProfileStore(s => s.recordFeatureUse)
  const location = useLocation()

  // Track which tracks we've already counted (prevent double-counting)
  const countedRef = useRef<Set<string>>(new Set())
  const lastTrackRef = useRef<string | null>(null)

  // Observe track plays: count after 30s of playback
  useEffect(() => {
    if (!currentTrack || !isPlaying) return

    const trackKey = `${currentTrack.id}-${currentTrack.title}`

    // Reset tracking on track change
    if (trackKey !== lastTrackRef.current) {
      lastTrackRef.current = trackKey
    }

    // Count the play once position passes threshold
    if (position >= MIN_PLAY_SECONDS && !countedRef.current.has(trackKey)) {
      countedRef.current.add(trackKey)
      recordPlay({
        artist: currentTrack.artist,
        genre: undefined, // Genre comes from search results, not track type currently
        duration: currentTrack.duration,
      })

      // Keep the set from growing unbounded
      if (countedRef.current.size > 100) {
        const arr = Array.from(countedRef.current)
        countedRef.current = new Set(arr.slice(-50))
      }
    }
  }, [currentTrack, isPlaying, position, recordPlay])

  // Observe navigation
  useEffect(() => {
    const basePath = '/' + (location.pathname.split('/')[1] ?? '')
    const feature = ROUTE_FEATURES[basePath]
    if (feature) {
      recordFeatureUse(feature)
    }
  }, [location.pathname, recordFeatureUse])

  // Run decay once per session
  useEffect(() => {
    useListeningProfileStore.getState().runDecay()
  }, [])
}
