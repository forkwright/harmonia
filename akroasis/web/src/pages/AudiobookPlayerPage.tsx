import { useState, useEffect, useRef, useCallback } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useAudiobookStore } from '../stores/audiobookStore'
import { usePlayerStore } from '../stores/playerStore'
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer'
import { apiClient } from '../api/client'
import { syncService } from '../services/syncService'
import { Button } from '../components/Button'
import { useArtworkViewer } from '../stores/artworkViewerStore'
import type { Bookmark, Chapter } from '../types'

function formatTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
  }
  return `${minutes}:${seconds.toString().padStart(2, '0')}`
}

function formatDuration(minutes?: number): string {
  if (!minutes) return ''
  const hours = Math.floor(minutes / 60)
  const mins = minutes % 60
  return hours > 0 ? `${hours}h ${mins}m` : `${mins}m`
}

function formatCountdown(ms: number): string {
  const totalMinutes = Math.ceil(ms / 60000)
  if (totalMinutes >= 60) {
    const h = Math.floor(totalMinutes / 60)
    const m = totalMinutes % 60
    return `${h}h ${m}m`
  }
  return `${totalMinutes}m`
}

const SPEED_PRESETS = [0.75, 1, 1.25, 1.5, 2]

function SleepTimerMenu({
  sleepTimerTarget,
  sleepTimerMode,
  onSet,
  onClear,
}: {
  sleepTimerTarget: number | null
  sleepTimerMode: 'minutes' | 'end-of-chapter' | null
  onSet: (value: number | 'end-of-chapter') => void
  onClear: () => void
}) {
  const [open, setOpen] = useState(false)
  const [remaining, setRemaining] = useState<number | null>(null)
  const isActive = sleepTimerMode !== null

  useEffect(() => {
    if (!sleepTimerTarget) return
    const update = () => setRemaining(Math.max(0, sleepTimerTarget - Date.now()))
    update()
    const id = setInterval(update, 1000)
    return () => clearInterval(id)
  }, [sleepTimerTarget])

  return (
    <div className="relative">
      <button
        onClick={() => setOpen(!open)}
        className={`relative p-2 rounded transition-colors ${
          isActive ? 'text-theme-primary bg-surface-sunken' : 'text-theme-tertiary hover:text-theme-primary'
        }`}
        aria-label="Sleep timer"
      >
        <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
        </svg>
        {isActive && remaining !== null && (
          <span className="absolute -top-1 -right-1 text-[10px] bg-accent text-theme-primary px-1 rounded-full leading-tight">
            {formatCountdown(remaining)}
          </span>
        )}
        {isActive && sleepTimerMode === 'end-of-chapter' && (
          <span className="absolute -top-1 -right-1 text-[10px] bg-accent text-theme-primary px-1 rounded-full leading-tight">
            Ch
          </span>
        )}
      </button>
      {open && (
        <div className="absolute bottom-full mb-2 right-0 bg-surface-sunken border border-theme-default rounded-lg shadow-xl p-2 min-w-[140px] z-10">
          {isActive ? (
            <button
              onClick={() => { onClear(); setOpen(false) }}
              className="w-full text-left px-3 py-1.5 text-sm text-red-400 hover:bg-accent-subtle rounded"
            >
              Cancel timer
            </button>
          ) : (
            <>
              {[15, 30, 45, 60].map((m) => (
                <button
                  key={m}
                  onClick={() => { onSet(m); setOpen(false) }}
                  className="w-full text-left px-3 py-1.5 text-sm text-theme-primary hover:bg-accent-subtle rounded"
                >
                  {m} min
                </button>
              ))}
              <button
                onClick={() => { onSet('end-of-chapter'); setOpen(false) }}
                className="w-full text-left px-3 py-1.5 text-sm text-theme-primary hover:bg-accent-subtle rounded"
              >
                End of chapter
              </button>
            </>
          )}
        </div>
      )}
    </div>
  )
}

function SpeedControl({
  speed,
  onSpeedChange,
}: {
  speed: number
  onSpeedChange: (speed: number) => void
}) {
  const [open, setOpen] = useState(false)

  return (
    <div className="relative">
      <button
        onClick={() => setOpen(!open)}
        className={`px-2 py-1 text-sm rounded transition-colors ${
          speed !== 1 ? 'text-theme-primary bg-surface-sunken font-medium' : 'text-theme-tertiary hover:text-theme-primary'
        }`}
        aria-label="Playback speed"
      >
        {speed}x
      </button>
      {open && (
        <div className="absolute bottom-full mb-2 right-0 bg-surface-sunken border border-theme-default rounded-lg shadow-xl p-1 z-10">
          {SPEED_PRESETS.map((s) => (
            <button
              key={s}
              onClick={() => { onSpeedChange(s); setOpen(false) }}
              className={`block w-full text-left px-3 py-1.5 text-sm rounded ${
                s === speed ? 'text-theme-primary bg-surface-sunken' : 'text-theme-secondary hover:bg-accent-subtle'
              }`}
            >
              {s}x
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

function BookmarkList({
  bookmarks,
  onSeek,
  onRemove,
}: {
  bookmarks: Bookmark[]
  onSeek: (positionMs: number) => void
  onRemove: (id: string) => void
}) {
  if (bookmarks.length === 0) return null

  return (
    <div className="mt-6">
      <h3 className="text-sm font-semibold text-theme-tertiary uppercase tracking-wider mb-3">Bookmarks</h3>
      <div className="space-y-1">
        {bookmarks.map((bm) => (
          <div
            key={bm.id}
            className="flex items-center gap-3 px-3 py-2 rounded hover:bg-accent-subtle group"
          >
            <button
              onClick={() => onSeek(bm.positionMs)}
              className="flex-1 text-left min-w-0"
            >
              <div className="flex items-center gap-2">
                <span className="text-xs text-theme-tertiary">{formatTime(bm.positionMs)}</span>
                {bm.chapterTitle && (
                  <span className="text-xs text-theme-tertiary truncate">{bm.chapterTitle}</span>
                )}
              </div>
              {bm.note && <p className="text-sm text-theme-secondary truncate mt-0.5">{bm.note}</p>}
            </button>
            <button
              onClick={() => onRemove(bm.id)}
              className="text-theme-muted hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0"
              aria-label="Remove bookmark"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        ))}
      </div>
    </div>
  )
}

function ChapterList({
  chapters,
  currentChapter,
  onSelect,
}: {
  chapters: Chapter[]
  currentChapter: Chapter | null
  onSelect: (chapter: Chapter) => void
}) {
  const listRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (currentChapter && listRef.current) {
      const active = listRef.current.querySelector('[data-active="true"]')
      active?.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
    }
  }, [currentChapter])

  if (chapters.length === 0) return null

  return (
    <div className="mt-6">
      <h3 className="text-sm font-semibold text-theme-tertiary uppercase tracking-wider mb-3">Chapters</h3>
      <div ref={listRef} className="max-h-64 overflow-y-auto space-y-1 pr-1">
        {chapters.map((chapter) => {
          const isCurrent = currentChapter?.index === chapter.index
          const duration = chapter.endTimeMs - chapter.startTimeMs
          return (
            <button
              key={chapter.index}
              data-active={isCurrent}
              onClick={() => onSelect(chapter)}
              className={`w-full text-left flex items-center gap-3 px-3 py-2 rounded transition-colors ${
                isCurrent
                  ? 'bg-surface-sunken text-theme-primary'
                  : 'hover:bg-accent-subtle text-theme-secondary'
              }`}
            >
              <span className="text-xs text-theme-tertiary w-6 text-right flex-shrink-0">
                {chapter.index + 1}
              </span>
              <span className="flex-1 truncate text-sm">{chapter.title}</span>
              <span className="text-xs text-theme-tertiary flex-shrink-0">
                {formatTime(duration)}
              </span>
            </button>
          )
        })}
      </div>
    </div>
  )
}

export function AudiobookPlayerPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const prevChapterRef = useRef<number | null>(null)

  const {
    currentAudiobook,
    chapters,
    currentChapter,
    positionMs,
    isPlaying,
    loading,
    error,
    sleepTimerTarget,
    sleepTimerMode,
    loadChapters,
    playAudiobook,
    setChapter,
    setPosition,
    setIsPlaying,
    setSleepTimer,
    clearSleepTimer,
    getBookSpeed,
    setBookSpeed,
    addBookmark,
    removeBookmark,
    getBookmarksForBook,
  } = useAudiobookStore()

  const { setPlaybackSpeed } = usePlayerStore()
  const { togglePlayPause, seek } = useWebAudioPlayer()
  const openArtwork = useArtworkViewer((s) => s.open)

  const audiobookId = currentAudiobook?.id
  const currentSpeed = audiobookId ? getBookSpeed(audiobookId) : 1
  const currentBookmarks = audiobookId ? getBookmarksForBook(audiobookId) : []

  // Auto-sync progress (replaces manual 30s interval)
  useEffect(() => {
    if (!currentAudiobook) return
    const totalMs = (currentAudiobook.metadata.durationMinutes ?? 0) * 60 * 1000
    return syncService.startAutoSync(() => {
      const state = useAudiobookStore.getState()
      if (!state.currentAudiobook || !state.isPlaying) return null
      return {
        mediaItemId: state.currentAudiobook.id,
        positionMs: state.positionMs,
        totalDurationMs: totalMs,
      }
    })
  }, [currentAudiobook?.id]) // eslint-disable-line react-hooks/exhaustive-deps

  // Load audiobook on mount
  useEffect(() => {
    if (!id) return
    const abId = Number(id)

    async function load() {
      try {
        const audiobook = await apiClient.getAudiobook(abId)
        playAudiobook(audiobook)
        await loadChapters(abId)

        // Restore saved speed
        const savedSpeed = useAudiobookStore.getState().getBookSpeed(abId)
        if (savedSpeed !== 1) setPlaybackSpeed(savedSpeed)

        try {
          const progress = await apiClient.getProgress(abId)
          if (progress && !progress.isComplete) {
            setPosition(progress.positionMs)
          }
        } catch {
          // No saved progress
        }
      } catch {
        // Error handled by store
      }
    }

    load()

    return () => {
      setPlaybackSpeed(1)
    }
  }, [id, playAudiobook, loadChapters, setPosition, setPlaybackSpeed])

  // Sleep timer — minutes mode
  useEffect(() => {
    if (sleepTimerMode !== 'minutes' || !sleepTimerTarget) return
    const check = () => {
      if (Date.now() >= sleepTimerTarget) {
        setIsPlaying(false)
        togglePlayPause()
        clearSleepTimer()
      }
    }
    const id = setInterval(check, 1000)
    return () => clearInterval(id)
  }, [sleepTimerMode, sleepTimerTarget, setIsPlaying, togglePlayPause, clearSleepTimer])

  // Sleep timer — end-of-chapter mode
  useEffect(() => {
    if (sleepTimerMode !== 'end-of-chapter' || !currentChapter) return
    const prevIdx = prevChapterRef.current
    if (prevIdx !== null && prevIdx !== currentChapter.index) {
      setIsPlaying(false)
      togglePlayPause()
      clearSleepTimer()
    }
    prevChapterRef.current = currentChapter.index
  }, [sleepTimerMode, currentChapter, setIsPlaying, togglePlayPause, clearSleepTimer])

  // Track chapter index for end-of-chapter detection
  useEffect(() => {
    if (currentChapter) {
      prevChapterRef.current = currentChapter.index
    }
  }, [currentChapter])

  const handleSeek = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const ms = Number(e.target.value)
    setPosition(ms)
    seek(ms / 1000)
  }, [setPosition, seek])

  const handleChapterSelect = useCallback((chapter: Chapter) => {
    setChapter(chapter)
    seek(chapter.startTimeMs / 1000)
  }, [setChapter, seek])

  const handleSkip = useCallback((deltaMs: number) => {
    const newPos = Math.max(0, positionMs + deltaMs)
    setPosition(newPos)
    seek(newPos / 1000)
  }, [positionMs, setPosition, seek])

  const handlePlayPause = useCallback(() => {
    setIsPlaying(!isPlaying)
    togglePlayPause()
  }, [isPlaying, setIsPlaying, togglePlayPause])

  const handleSpeedChange = useCallback((speed: number) => {
    setPlaybackSpeed(speed)
    if (audiobookId) setBookSpeed(audiobookId, speed)
  }, [audiobookId, setPlaybackSpeed, setBookSpeed])

  const handleBookmarkSeek = useCallback((posMs: number) => {
    setPosition(posMs)
    seek(posMs / 1000)
  }, [setPosition, seek])

  if (loading) {
    return <div className="flex items-center justify-center min-h-screen text-theme-tertiary">Loading...</div>
  }

  if (error || !currentAudiobook) {
    return (
      <div className="flex flex-col items-center justify-center min-h-screen gap-4">
        <p className="text-red-400">{error || 'Audiobook not found'}</p>
        <Button onClick={() => navigate('/audiobooks')}>Back to Library</Button>
      </div>
    )
  }

  const totalMs = (currentAudiobook.metadata.durationMinutes ?? 0) * 60 * 1000
  const coverUrl = apiClient.getAudiobookCoverUrl(currentAudiobook.id, 400)

  return (
    <div className="min-h-screen flex items-start justify-center p-4 pt-8">
      <div className="w-full max-w-2xl">
        {/* Back button */}
        <Button variant="ghost" onClick={() => navigate('/audiobooks')} className="mb-4">
          ← Audiobooks
        </Button>

        {/* Cover + Info */}
        <div className="text-center mb-6">
          <div className="w-48 h-48 mx-auto mb-4 bg-surface-sunken rounded-lg overflow-hidden shadow-xl">
            <img
              src={coverUrl}
              alt={currentAudiobook.title}
              className="w-full h-full object-cover cursor-zoom-in"
              onClick={() => openArtwork(apiClient.getAudiobookCoverUrl(currentAudiobook.id))}
              title="Click to view full size"
              onError={(e) => {
                const el = e.target as HTMLImageElement
                el.style.display = 'none'
              }}
            />
          </div>
          <h1 className="text-2xl font-serif font-semibold" style={{ color: 'rgb(var(--text-primary))' }}>{currentAudiobook.title}</h1>
          {currentAudiobook.metadata.narrator && (
            <p className="text-theme-tertiary mt-1">
              Narrated by {currentAudiobook.metadata.narrators.length > 1
                ? currentAudiobook.metadata.narrators.join(', ')
                : currentAudiobook.metadata.narrator}
            </p>
          )}
          <div className="flex items-center justify-center gap-3 mt-2 text-sm text-theme-tertiary">
            {currentAudiobook.year > 0 && <span>{currentAudiobook.year}</span>}
            <span>{formatDuration(currentAudiobook.metadata.durationMinutes)}</span>
            {currentAudiobook.metadata.isAbridged && (
              <span className="px-1.5 py-0.5 bg-surface-sunken rounded text-theme-secondary text-xs">Abridged</span>
            )}
          </div>
          {currentChapter && (
            <p className="text-sm text-theme-secondary mt-3 font-medium">
              {currentChapter.title}
            </p>
          )}
        </div>

        {/* Progress bar */}
        <div className="mb-4">
          <input
            type="range"
            min={0}
            max={totalMs}
            value={positionMs}
            onChange={handleSeek}
            className="w-full h-2 bg-surface-sunken rounded-lg appearance-none cursor-pointer"
            style={{
              backgroundImage: totalMs > 0
                ? `linear-gradient(to right, rgb(180, 111, 63) 0%, rgb(180, 111, 63) ${(positionMs / totalMs) * 100}%, rgb(37, 28, 23) ${(positionMs / totalMs) * 100}%, rgb(37, 28, 23) 100%)`
                : undefined,
            }}
          />
          <div className="flex justify-between text-sm text-theme-tertiary mt-1">
            <span>{formatTime(positionMs)}</span>
            <span>-{formatTime(Math.max(0, totalMs - positionMs))}</span>
          </div>
        </div>

        {/* Controls */}
        <div className="flex items-center justify-center gap-6 mb-4">
          <button
            onClick={() => handleSkip(-30000)}
            className="text-theme-tertiary hover:text-theme-primary transition-colors"
            aria-label="Skip back 30 seconds"
          >
            <svg className="w-8 h-8" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M12.066 11.2a1 1 0 000 1.6l5.334 4A1 1 0 0019 16V8a1 1 0 00-1.6-.8l-5.333 4zM4.066 11.2a1 1 0 000 1.6l5.334 4A1 1 0 0011 16V8a1 1 0 00-1.6-.8l-5.334 4z" />
            </svg>
            <span className="text-xs block mt-0.5">30s</span>
          </button>

          <button
            onClick={handlePlayPause}
            className="w-16 h-16 flex items-center justify-center rounded-full bg-accent hover:bg-accent text-theme-primary transition-colors"
            aria-label={isPlaying ? 'Pause' : 'Play'}
          >
            {isPlaying ? (
              <svg className="w-8 h-8" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z" clipRule="evenodd" />
              </svg>
            ) : (
              <svg className="w-8 h-8" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd" />
              </svg>
            )}
          </button>

          <button
            onClick={() => handleSkip(30000)}
            className="text-theme-tertiary hover:text-theme-primary transition-colors"
            aria-label="Skip forward 30 seconds"
          >
            <svg className="w-8 h-8" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M11.933 12.8a1 1 0 000-1.6L6.6 7.2A1 1 0 005 8v8a1 1 0 001.6.8l5.333-4zM19.933 12.8a1 1 0 000-1.6l-5.333-4A1 1 0 0013 8v8a1 1 0 001.6.8l5.333-4z" />
            </svg>
            <span className="text-xs block mt-0.5">30s</span>
          </button>
        </div>

        {/* Secondary controls: bookmark, speed, sleep timer */}
        <div className="flex items-center justify-center gap-4 mb-6">
          <button
            onClick={() => addBookmark()}
            className="p-2 text-theme-tertiary hover:text-theme-primary transition-colors"
            aria-label="Add bookmark"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
            </svg>
          </button>

          <SpeedControl speed={currentSpeed} onSpeedChange={handleSpeedChange} />

          <SleepTimerMenu
            sleepTimerTarget={sleepTimerTarget}
            sleepTimerMode={sleepTimerMode}
            onSet={setSleepTimer}
            onClear={clearSleepTimer}
          />
        </div>

        {/* Description */}
        {currentAudiobook.metadata.description && (
          <div className="mb-6">
            <h3 className="text-sm font-semibold text-theme-tertiary uppercase tracking-wider mb-2">About</h3>
            <p className="text-sm text-theme-secondary leading-relaxed">
              {currentAudiobook.metadata.description}
            </p>
          </div>
        )}

        {/* Chapters */}
        <ChapterList
          chapters={chapters}
          currentChapter={currentChapter}
          onSelect={handleChapterSelect}
        />

        {/* Bookmarks */}
        <BookmarkList
          bookmarks={currentBookmarks}
          onSeek={handleBookmarkSeek}
          onRemove={removeBookmark}
        />
      </div>
    </div>
  )
}
