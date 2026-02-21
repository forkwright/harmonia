import { useEffect, useRef, useCallback } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useAudiobookStore } from '../stores/audiobookStore'
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer'
import { apiClient } from '../api/client'
import { Button } from '../components/Button'
import type { Chapter } from '../types'

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
      <h3 className="text-sm font-semibold text-bronze-400 uppercase tracking-wider mb-3">Chapters</h3>
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
                  ? 'bg-bronze-700 text-bronze-100'
                  : 'hover:bg-bronze-800/50 text-bronze-300'
              }`}
            >
              <span className="text-xs text-bronze-500 w-6 text-right flex-shrink-0">
                {chapter.index + 1}
              </span>
              <span className="flex-1 truncate text-sm">{chapter.title}</span>
              <span className="text-xs text-bronze-500 flex-shrink-0">
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
  const progressInterval = useRef<ReturnType<typeof setInterval> | null>(null)

  const {
    currentAudiobook,
    chapters,
    currentChapter,
    positionMs,
    isPlaying,
    loading,
    error,
    loadChapters,
    playAudiobook,
    setChapter,
    setPosition,
    setIsPlaying,
    saveProgress,
  } = useAudiobookStore()

  const { togglePlayPause, seek } = useWebAudioPlayer()

  // Load audiobook on mount
  useEffect(() => {
    if (!id) return
    const audiobookId = Number(id)

    async function load() {
      try {
        const audiobook = await apiClient.getAudiobook(audiobookId)
        playAudiobook(audiobook)

        // Load chapters — using the audiobook ID as media file ID for now
        await loadChapters(audiobookId)

        // Try to restore progress
        try {
          const progress = await apiClient.getProgress(audiobookId)
          if (progress && !progress.isComplete) {
            setPosition(progress.positionMs)
          }
        } catch {
          // No saved progress — start from beginning
        }
      } catch {
        // Error handled by store
      }
    }

    load()
  }, [id, playAudiobook, loadChapters, setPosition])

  // Save progress periodically
  useEffect(() => {
    if (isPlaying) {
      progressInterval.current = setInterval(() => {
        saveProgress()
      }, 30000) // Save every 30 seconds
    }
    return () => {
      if (progressInterval.current) {
        clearInterval(progressInterval.current)
        progressInterval.current = null
      }
    }
  }, [isPlaying, saveProgress])

  // Save on unmount
  useEffect(() => {
    return () => { saveProgress() }
  }, [saveProgress])

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

  if (loading) {
    return <div className="flex items-center justify-center min-h-screen text-bronze-400">Loading...</div>
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
          <div className="w-48 h-48 mx-auto mb-4 bg-bronze-800 rounded-lg overflow-hidden shadow-xl">
            <img
              src={coverUrl}
              alt={currentAudiobook.title}
              className="w-full h-full object-cover"
              onError={(e) => {
                const el = e.target as HTMLImageElement
                el.style.display = 'none'
              }}
            />
          </div>
          <h1 className="text-2xl font-bold text-bronze-100">{currentAudiobook.title}</h1>
          {currentAudiobook.metadata.narrator && (
            <p className="text-bronze-400 mt-1">
              Narrated by {currentAudiobook.metadata.narrators.length > 1
                ? currentAudiobook.metadata.narrators.join(', ')
                : currentAudiobook.metadata.narrator}
            </p>
          )}
          <div className="flex items-center justify-center gap-3 mt-2 text-sm text-bronze-500">
            {currentAudiobook.year > 0 && <span>{currentAudiobook.year}</span>}
            <span>{formatDuration(currentAudiobook.metadata.durationMinutes)}</span>
            {currentAudiobook.metadata.isAbridged && (
              <span className="px-1.5 py-0.5 bg-bronze-700 rounded text-bronze-300 text-xs">Abridged</span>
            )}
          </div>
          {currentChapter && (
            <p className="text-sm text-bronze-300 mt-3 font-medium">
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
            className="w-full h-2 bg-bronze-800 rounded-lg appearance-none cursor-pointer"
            style={{
              backgroundImage: totalMs > 0
                ? `linear-gradient(to right, rgb(180, 111, 63) 0%, rgb(180, 111, 63) ${(positionMs / totalMs) * 100}%, rgb(37, 28, 23) ${(positionMs / totalMs) * 100}%, rgb(37, 28, 23) 100%)`
                : undefined,
            }}
          />
          <div className="flex justify-between text-sm text-bronze-500 mt-1">
            <span>{formatTime(positionMs)}</span>
            <span>-{formatTime(Math.max(0, totalMs - positionMs))}</span>
          </div>
        </div>

        {/* Controls */}
        <div className="flex items-center justify-center gap-6 mb-6">
          <button
            onClick={() => handleSkip(-30000)}
            className="text-bronze-400 hover:text-bronze-200 transition-colors"
            aria-label="Skip back 30 seconds"
          >
            <svg className="w-8 h-8" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M12.066 11.2a1 1 0 000 1.6l5.334 4A1 1 0 0019 16V8a1 1 0 00-1.6-.8l-5.333 4zM4.066 11.2a1 1 0 000 1.6l5.334 4A1 1 0 0011 16V8a1 1 0 00-1.6-.8l-5.334 4z" />
            </svg>
            <span className="text-xs block mt-0.5">30s</span>
          </button>

          <button
            onClick={handlePlayPause}
            className="w-16 h-16 flex items-center justify-center rounded-full bg-bronze-600 hover:bg-bronze-500 text-bronze-100 transition-colors"
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
            className="text-bronze-400 hover:text-bronze-200 transition-colors"
            aria-label="Skip forward 30 seconds"
          >
            <svg className="w-8 h-8" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M11.933 12.8a1 1 0 000-1.6L6.6 7.2A1 1 0 005 8v8a1 1 0 001.6.8l5.333-4zM19.933 12.8a1 1 0 000-1.6l-5.333-4A1 1 0 0013 8v8a1 1 0 001.6.8l5.333-4z" />
            </svg>
            <span className="text-xs block mt-0.5">30s</span>
          </button>
        </div>

        {/* Description */}
        {currentAudiobook.metadata.description && (
          <div className="mb-6">
            <h3 className="text-sm font-semibold text-bronze-400 uppercase tracking-wider mb-2">About</h3>
            <p className="text-sm text-bronze-300 leading-relaxed">
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
      </div>
    </div>
  )
}
