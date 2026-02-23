// Podcast player surface — asymmetric skip, speed control, show notes, mark played
import { useState } from 'react'
import { usePlayerStore } from '../stores/playerStore'
import { usePodcastStore } from '../stores/podcastStore'
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer'
import { useArtworkViewer } from '../stores/artworkViewerStore'

const SPEED_PRESETS = [1, 1.25, 1.5, 1.75, 2]
const SKIP_BACK = 15  // seconds
const SKIP_FORWARD = 30  // seconds

function formatTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  if (hours > 0) return `${hours}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
  return `${minutes}:${seconds.toString().padStart(2, '0')}`
}

export function PodcastPlayer() {
  const openArtwork = useArtworkViewer((s) => s.open)
  const { isPlaying, position, duration, volume, setVolume } = usePlayerStore()
  const { currentEpisode, currentShow } = usePodcastStore()
  const { togglePlayPause, seek } = useWebAudioPlayer()
  const [speed, setSpeed] = useState(1)
  const [showNotes, setShowNotes] = useState(false)

  if (!currentEpisode || !currentShow) return null

  const coverUrl = currentEpisode.imageUrl ?? currentShow.imageUrl ?? null

  const handleSkipBack = () => {
    const newPos = Math.max(0, (position / 1000) - SKIP_BACK)
    seek(newPos)
  }

  const handleSkipForward = () => {
    const maxPos = duration / 1000
    const newPos = Math.min(maxPos, (position / 1000) + SKIP_FORWARD)
    seek(newPos)
  }

  const handleSpeedCycle = () => {
    const currentIdx = SPEED_PRESETS.indexOf(speed)
    const nextIdx = (currentIdx + 1) % SPEED_PRESETS.length
    setSpeed(SPEED_PRESETS[nextIdx])
    // TODO: Wire to Web Audio playbackRate when available
  }

  const handleSeek = (e: React.ChangeEvent<HTMLInputElement>) => {
    seek(Number(e.target.value) / 1000)
  }

  const progress = duration > 0 ? (position / duration) * 100 : 0

  return (
    <div className="w-full max-w-lg">
      {/* Show art + info */}
      <div className="flex gap-5 mb-8">
        <div
          className="w-28 h-28 rounded-xl overflow-hidden bg-surface-raised flex-shrink-0 shadow-lg"
          role={coverUrl ? 'button' : undefined}
          onClick={coverUrl ? () => openArtwork(coverUrl) : undefined}
          style={coverUrl ? { cursor: 'zoom-in' } : undefined}
        >
          {coverUrl ? (
            <img src={coverUrl} alt={currentShow.title} className="w-full h-full object-cover" />
          ) : (
            <div className="w-full h-full flex items-center justify-center">
              <svg className="w-12 h-12 text-theme-muted" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217z" clipRule="evenodd"/>
              </svg>
            </div>
          )}
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-theme-tertiary text-xs uppercase tracking-wider">{currentShow.title}</p>
          <h1 className="text-xl font-bold text-theme-primary leading-tight mt-1 line-clamp-3">
            {currentEpisode.title}
          </h1>
          {currentShow.author && (
            <p className="text-theme-tertiary text-sm mt-1">{currentShow.author}</p>
          )}
        </div>
      </div>

      {/* Progress bar */}
      <div className="mb-6">
        <input
          type="range"
          min="0"
          max={duration || 100}
          value={position}
          onChange={handleSeek}
          className="w-full"
          aria-label="Seek"
        />
        <div className="flex justify-between text-xs text-theme-tertiary mt-1.5 tabular-nums">
          <span>{formatTime(position)}</span>
          <span>-{formatTime(Math.max(0, duration - position))}</span>
        </div>
      </div>

      {/* Transport — asymmetric skip */}
      <div className="flex items-center justify-center gap-8 mb-6">
        <button
          onClick={handleSkipBack}
          className="relative p-3 text-theme-tertiary hover:text-theme-primary transition-colors"
          aria-label={`Skip back ${SKIP_BACK} seconds`}
        >
          <svg className="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M12.066 11.2a1 1 0 000 1.6l5.334 4A1 1 0 0019 16V8a1 1 0 00-1.6-.8l-5.333 4zM4.066 11.2a1 1 0 000 1.6l5.334 4A1 1 0 0011 16V8a1 1 0 00-1.6-.8l-5.334 4z" />
          </svg>
          <span className="absolute -bottom-1 left-1/2 -translate-x-1/2 text-[10px] text-theme-muted">{SKIP_BACK}</span>
        </button>

        <button
          onClick={togglePlayPause}
          className="w-16 h-16 flex items-center justify-center rounded-full bg-accent text-surface-base hover:bg-white transition-colors"
          aria-label={isPlaying ? 'Pause' : 'Play'}
        >
          {isPlaying ? (
            <svg className="w-7 h-7" fill="currentColor" viewBox="0 0 20 20"><path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z" clipRule="evenodd"/></svg>
          ) : (
            <svg className="w-7 h-7 ml-0.5" fill="currentColor" viewBox="0 0 20 20"><path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/></svg>
          )}
        </button>

        <button
          onClick={handleSkipForward}
          className="relative p-3 text-theme-tertiary hover:text-theme-primary transition-colors"
          aria-label={`Skip forward ${SKIP_FORWARD} seconds`}
        >
          <svg className="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M11.933 12.8a1 1 0 000-1.6L6.6 7.2A1 1 0 005 8v8a1 1 0 001.6.8l5.333-4zM19.933 12.8a1 1 0 000-1.6l-5.333-4A1 1 0 0013 8v8a1 1 0 001.6.8l5.333-4z" />
          </svg>
          <span className="absolute -bottom-1 left-1/2 -translate-x-1/2 text-[10px] text-theme-muted">{SKIP_FORWARD}</span>
        </button>
      </div>

      {/* Speed + Volume row */}
      <div className="flex items-center justify-between mb-8">
        <button
          onClick={handleSpeedCycle}
          className="px-3 py-1.5 text-sm font-medium rounded-lg
            bg-surface-raised/80 border border-theme-subtle
            text-theme-secondary hover:text-theme-primary hover:border-theme-default
            transition-colors tabular-nums"
          aria-label={`Playback speed ${speed}x`}
        >
          {speed}×
        </button>

        <div className="flex items-center gap-3 flex-1 ml-6">
          <svg className="w-4 h-4 text-theme-muted flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
            <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217z" clipRule="evenodd"/>
          </svg>
          <input
            type="range"
            min="0"
            max="100"
            value={volume * 100}
            onChange={(e) => setVolume(Number.parseFloat(e.target.value) / 100)}
            className="flex-1"
          />
          <span className="text-xs text-theme-tertiary w-10 text-right tabular-nums">{Math.round(volume * 100)}%</span>
        </div>
      </div>

      {/* Episode progress */}
      {duration > 0 && (
        <div className="mb-6 p-3 bg-surface-raised/60 rounded-lg">
          <div className="flex items-center justify-between text-xs text-theme-tertiary mb-1.5">
            <span>Episode progress</span>
            <span className="tabular-nums">{Math.round(progress)}%</span>
          </div>
          <div className="w-full h-1 rounded-full bg-accent-subtle">
            <div
              className="h-full rounded-full bg-[rgb(var(--accent-primary))] transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      )}

      {/* Show notes */}
      {currentEpisode.description && (
        <div className="border-t border-theme-subtle">
          <button
            onClick={() => setShowNotes(!showNotes)}
            className="w-full flex items-center gap-2 py-3 text-sm text-theme-tertiary hover:text-theme-primary transition-colors"
          >
            <svg className={`w-3.5 h-3.5 transition-transform ${showNotes ? 'rotate-90' : ''}`} fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z" clipRule="evenodd" />
            </svg>
            <span>Show Notes</span>
          </button>
          {showNotes && (
            <div className="pb-4 animate-[fadeIn_150ms_ease-out] text-sm text-theme-tertiary leading-relaxed whitespace-pre-wrap">
              {currentEpisode.description}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
