// Lyrics state: fetch, cache, and active-line tracking
import { useEffect, useRef } from 'react'
import { create } from 'zustand'
import type { Track } from '../types'
import { fetchLrclib } from '../api/lrclib'
import { parseLrc, findActiveLine } from '../utils/lrcParser'
import type { LrcLine } from '../utils/lrcParser'

export type LyricsStatus = 'idle' | 'loading' | 'synced' | 'plain' | 'not-found' | 'error'

interface LyricsState {
  status: LyricsStatus
  lines: LrcLine[]
  plainLyrics: string | null
  activeLine: number
  trackId: number | null

  _setStatus: (status: LyricsStatus) => void
  _setContent: (lines: LrcLine[], plain: string | null, trackId: number) => void
  _setActiveLine: (index: number) => void
  _reset: () => void
}

const useLyricsStore = create<LyricsState>((set) => ({
  status: 'idle',
  lines: [],
  plainLyrics: null,
  activeLine: -1,
  trackId: null,

  _setStatus: (status) => set({ status }),
  _setContent: (lines, plain, trackId) =>
    set({ lines, plainLyrics: plain, trackId, activeLine: -1 }),
  _setActiveLine: (activeLine) => set({ activeLine }),
  _reset: () =>
    set({ status: 'idle', lines: [], plainLyrics: null, activeLine: -1, trackId: null }),
}))

// Cache keyed by "<id>" to avoid re-fetching
const cache = new Map<
  number,
  { lines: LrcLine[]; plain: string | null; status: 'synced' | 'plain' | 'not-found' }
>()

export function useLyrics(track: Track | null, positionMs: number) {
  const {
    status,
    lines,
    plainLyrics,
    activeLine,
    trackId,
    _setStatus,
    _setContent,
    _setActiveLine,
    _reset,
  } = useLyricsStore()

  const fetchingRef = useRef<number | null>(null)

  // Fetch when track changes
  useEffect(() => {
    if (!track) {
      _reset()
      return
    }

    if (track.id === trackId) return

    const hit = cache.get(track.id)
    if (hit) {
      _setContent(hit.lines, hit.plain, track.id)
      _setStatus(hit.status)
      return
    }

    fetchingRef.current = track.id
    _setStatus('loading')

    fetchLrclib(track.artist, track.title, track.album, track.duration)
      .then((result) => {
        if (fetchingRef.current !== track.id) return

        if (!result || (!result.syncedLyrics && !result.plainLyrics)) {
          cache.set(track.id, { lines: [], plain: null, status: 'not-found' })
          _setContent([], null, track.id)
          _setStatus('not-found')
          return
        }

        if (result.syncedLyrics) {
          const parsed = parseLrc(result.syncedLyrics)
          cache.set(track.id, { lines: parsed, plain: result.plainLyrics, status: 'synced' })
          _setContent(parsed, result.plainLyrics, track.id)
          _setStatus('synced')
        } else {
          cache.set(track.id, { lines: [], plain: result.plainLyrics, status: 'plain' })
          _setContent([], result.plainLyrics, track.id)
          _setStatus('plain')
        }
      })
      .catch(() => {
        if (fetchingRef.current !== track.id) return
        _setStatus('error')
      })
  }, [track?.id]) // eslint-disable-line react-hooks/exhaustive-deps

  // Update active line as position advances
  useEffect(() => {
    if (status !== 'synced' || lines.length === 0) return
    const next = findActiveLine(lines, positionMs)
    if (next !== activeLine) {
      _setActiveLine(next)
    }
  }, [positionMs, lines, status, activeLine, _setActiveLine])

  return { status, lines, plainLyrics, activeLine }
}
