import { useNavigate, useLocation } from 'react-router-dom'
import { usePlayerStore } from '../stores/playerStore'
import { usePodcastStore } from '../stores/podcastStore'
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer'
import { getCoverArtUrl } from '../api/client'

function formatTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  return `${minutes}:${seconds.toString().padStart(2, '0')}`
}

export function MiniPlayer() {
  const navigate = useNavigate()
  const location = useLocation()
  const { currentTrack, isPlaying, position, duration } = usePlayerStore()
  const { currentEpisode, currentShow } = usePodcastStore()
  const { togglePlayPause } = useWebAudioPlayer()

  const isPodcast = !!currentEpisode && !!currentShow

  // Don't show on player page (it's the expanded view) or login
  const hiddenPaths = ['/player', '/login']
  if (hiddenPaths.includes(location.pathname)) return null

  // Nothing playing
  if (!currentTrack && !isPodcast) return null

  const title = isPodcast ? currentEpisode.title : currentTrack?.title ?? ''
  const subtitle = isPodcast
    ? currentShow.title
    : [currentTrack?.artist, currentTrack?.album].filter(Boolean).join(' · ')

  const coverUrl = isPodcast
    ? (currentEpisode.imageUrl ?? currentShow.imageUrl)
    : currentTrack?.coverArtUrl
      ? getCoverArtUrl(currentTrack.id, 96)
      : null

  const progress = duration > 0 ? (position / duration) * 100 : 0

  return (
    <div className="fixed bottom-0 left-0 right-0 z-50">
      {/* Progress bar — sits on top edge of the bar */}
      <div className="h-0.5 bg-bronze-800">
        <div
          className="h-full bg-bronze-400 transition-[width] duration-300 ease-linear"
          style={{ width: `${progress}%` }}
        />
      </div>

      <div className="bg-bronze-950/95 backdrop-blur-md border-t border-bronze-800/50">
        <div className="max-w-7xl mx-auto px-4 py-2 flex items-center gap-3">
          {/* Album art — click to go to player */}
          <button
            onClick={() => navigate('/player')}
            className="flex-shrink-0 w-12 h-12 rounded-md overflow-hidden bg-bronze-800 hover:ring-2 hover:ring-bronze-500/50 transition-all"
            aria-label="Open player"
          >
            {coverUrl ? (
              <img src={coverUrl} alt={title} className="w-full h-full object-cover" />
            ) : (
              <div className="w-full h-full flex items-center justify-center">
                <svg className="w-6 h-6 text-bronze-600" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/>
                </svg>
              </div>
            )}
          </button>

          {/* Track info — click to go to player */}
          <button
            onClick={() => navigate('/player')}
            className="flex-1 min-w-0 text-left"
            aria-label="Open player"
          >
            <p className="text-sm font-medium text-bronze-100 truncate">{title}</p>
            <p className="text-xs text-bronze-400 truncate">{subtitle}</p>
          </button>

          {/* Time */}
          <span className="hidden sm:block text-xs text-bronze-500 tabular-nums flex-shrink-0">
            {formatTime(position)} / {formatTime(duration)}
          </span>

          {/* Play/Pause */}
          <button
            onClick={(e) => { e.stopPropagation(); togglePlayPause() }}
            className="flex-shrink-0 w-10 h-10 flex items-center justify-center rounded-full bg-bronze-800 hover:bg-bronze-700 text-bronze-100 transition-colors"
            aria-label={isPlaying ? 'Pause' : 'Play'}
          >
            {isPlaying ? (
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z" clipRule="evenodd" />
              </svg>
            ) : (
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd" />
              </svg>
            )}
          </button>
        </div>
      </div>
    </div>
  )
}
