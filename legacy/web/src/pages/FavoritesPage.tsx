// Liked Songs — virtual playlist backed by thymesisStore favorites
import { useState, useEffect, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { useThymesisStore } from '../stores/thymesisStore'
import { usePlayerStore } from '../stores/playerStore'
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer'
import { HeartButton } from '../components/HeartButton'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import type { Track } from '../types'

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = Math.floor(seconds % 60)
  return `${m}:${s.toString().padStart(2, '0')}`
}

export function FavoritesPage() {
  const navigate = useNavigate()
  const { getFavoriteTracks, favoriteIds } = useThymesisStore()
  const { setQueue, setCurrentTrack } = usePlayerStore()
  const { playTrack } = useWebAudioPlayer()
  const [tracks, setTracks] = useState<Track[]>([])
  const [loading, setLoading] = useState(true)
  const [page, setPage] = useState(1)
  const [hasMore, setHasMore] = useState(false)
  const [totalCount, setTotalCount] = useState(0)

  const loadTracks = useCallback(async (p = 1) => {
    setLoading(true)
    try {
      const result = await getFavoriteTracks(p, 50)
      setTracks(prev => p === 1 ? result.items : [...prev, ...result.items])
      setTotalCount(result.totalCount)
      setHasMore(p * 50 < result.totalCount)
      setPage(p)
    } catch {
      // Non-critical
    } finally {
      setLoading(false)
    }
  }, [getFavoriteTracks])

  useEffect(() => {
    void loadTracks(1)
  }, [loadTracks, favoriteIds.size])

  const handlePlay = (track: Track) => {
    setQueue(tracks)
    setCurrentTrack(track)
    playTrack(track)
  }

  const handlePlayAll = () => {
    if (tracks.length === 0) return
    setQueue(tracks)
    setCurrentTrack(tracks[0])
    playTrack(tracks[0])
  }

  return (
    <div className="container mx-auto p-4 max-w-4xl">
      <Card>
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <Button variant="ghost" size="sm" onClick={() => navigate('/playlists')}>
              ← Back
            </Button>
            <div>
              <h1 className="text-2xl font-serif font-semibold" style={{ color: 'rgb(var(--text-primary))' }}>
                <span className="inline-block mr-2">
                  <svg className="w-6 h-6 inline text-red-400" viewBox="0 0 20 20" fill="currentColor">
                    <path fillRule="evenodd" d="M3.172 5.172a4 4 0 015.656 0L10 6.343l1.172-1.171a4 4 0 115.656 5.656L10 17.657l-6.828-6.829a4 4 0 010-5.656z" clipRule="evenodd"/>
                  </svg>
                </span>
                Liked Songs
              </h1>
              <p className="text-sm text-theme-tertiary">{totalCount} songs</p>
            </div>
          </div>
          {tracks.length > 0 && (
            <Button variant="ghost" size="sm" onClick={handlePlayAll}>
              ▶ Play All
            </Button>
          )}
        </div>

        {loading && tracks.length === 0 && (
          <div className="text-center py-8 text-theme-tertiary">Loading...</div>
        )}

        {!loading && tracks.length === 0 && (
          <div className="text-center py-12">
            <svg className="w-16 h-16 text-theme-muted mx-auto mb-4" viewBox="0 0 20 20" fill="currentColor">
              <path fillRule="evenodd" d="M3.172 5.172a4 4 0 015.656 0L10 6.343l1.172-1.171a4 4 0 115.656 5.656L10 17.657l-6.828-6.829a4 4 0 010-5.656z" clipRule="evenodd"/>
            </svg>
            <p className="text-theme-tertiary">No liked songs yet</p>
            <p className="text-sm text-theme-muted mt-1">Tap the heart on any song to add it here</p>
          </div>
        )}

        <div className="space-y-1">
          {tracks.map((track, index) => (
            <div
              key={track.id}
              className="flex items-center gap-3 p-3 rounded-lg bg-surface-raised/60 hover:bg-accent-subtle transition-colors"
            >
              <button
                onClick={() => handlePlay(track)}
                className="text-theme-tertiary hover:text-theme-secondary p-1 min-w-[2rem] text-center"
              >
                <span className="text-sm font-mono">{(index + 1).toString().padStart(2, '0')}</span>
              </button>

              <div className="flex-1 min-w-0">
                <h3 className="text-theme-primary font-medium truncate">{track.title}</h3>
                <p className="text-sm text-theme-tertiary mt-0.5 truncate">
                  {track.artist} • {track.album}
                </p>
              </div>

              <span className="text-xs text-theme-muted tabular-nums">
                {formatDuration(track.duration)}
              </span>

              <HeartButton trackId={track.id} />
            </div>
          ))}
        </div>

        {hasMore && (
          <div className="text-center mt-4">
            <Button variant="ghost" size="sm" onClick={() => void loadTracks(page + 1)} disabled={loading}>
              {loading ? 'Loading...' : 'Load More'}
            </Button>
          </div>
        )}
      </Card>
    </div>
  )
}
