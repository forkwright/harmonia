import { useEffect, useRef, useState } from 'react'
import { usePlayerStore } from '../stores/playerStore'
import { Button } from '../components/Button'
import { Card } from '../components/Card'
import { apiClient } from '../api/client'

export function PlayerPage() {
  const audioRef = useRef<HTMLAudioElement>(null)
  const [error, setError] = useState('')

  const {
    currentTrack,
    isPlaying,
    position,
    duration,
    volume,
    setIsPlaying,
    setPosition,
    setDuration,
  } = usePlayerStore()

  useEffect(() => {
    const audio = audioRef.current
    if (!audio) return

    if (currentTrack) {
      audio.src = apiClient.getStreamUrl(currentTrack.id)
      if (isPlaying) {
        audio.play().catch((err) => {
          setError(err.message)
          setIsPlaying(false)
        })
      }
    }
  }, [currentTrack, isPlaying, setIsPlaying])

  useEffect(() => {
    const audio = audioRef.current
    if (!audio) return

    audio.volume = volume

    const handleTimeUpdate = () => setPosition(audio.currentTime * 1000)
    const handleLoadedMetadata = () => setDuration(audio.duration * 1000)
    const handleEnded = () => setIsPlaying(false)
    const handleError = () => {
      setError('Playback error')
      setIsPlaying(false)
    }

    audio.addEventListener('timeupdate', handleTimeUpdate)
    audio.addEventListener('loadedmetadata', handleLoadedMetadata)
    audio.addEventListener('ended', handleEnded)
    audio.addEventListener('error', handleError)

    return () => {
      audio.removeEventListener('timeupdate', handleTimeUpdate)
      audio.removeEventListener('loadedmetadata', handleLoadedMetadata)
      audio.removeEventListener('ended', handleEnded)
      audio.removeEventListener('error', handleError)
    }
  }, [setPosition, setDuration, setIsPlaying])

  const togglePlayPause = () => {
    const audio = audioRef.current
    if (!audio || !currentTrack) return

    if (isPlaying) {
      audio.pause()
      setIsPlaying(false)
    } else {
      audio.play().catch((err) => {
        setError(err.message)
      })
      setIsPlaying(true)
    }
  }

  const handleSeek = (e: React.ChangeEvent<HTMLInputElement>) => {
    const audio = audioRef.current
    if (!audio) return

    const seekTime = parseFloat(e.target.value)
    audio.currentTime = seekTime / 1000
    setPosition(seekTime)
  }

  const formatTime = (ms: number) => {
    const totalSeconds = Math.floor(ms / 1000)
    const minutes = Math.floor(totalSeconds / 60)
    const seconds = totalSeconds % 60
    return `${minutes}:${seconds.toString().padStart(2, '0')}`
  }

  return (
    <div className="min-h-screen flex items-center justify-center p-4">
      <audio ref={audioRef} />

      <div className="w-full max-w-2xl">
        <Card>
          {error && (
            <div className="mb-4 p-3 bg-red-900/50 border border-red-700 rounded-lg text-red-200 text-sm">
              {error}
            </div>
          )}

          <div className="text-center mb-6">
            <div className="w-64 h-64 mx-auto mb-6 bg-bronze-800 rounded-lg flex items-center justify-center">
              {currentTrack?.coverArtUrl ? (
                <img
                  src={apiClient.getCoverArtUrl(currentTrack.id, 256)}
                  alt={currentTrack.title}
                  className="w-full h-full object-cover rounded-lg"
                />
              ) : (
                <svg className="w-24 h-24 text-bronze-600" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/>
                </svg>
              )}
            </div>

            <h2 className="text-2xl font-bold text-bronze-100 mb-2">
              {currentTrack?.title || 'No track playing'}
            </h2>
            <p className="text-bronze-400">
              {currentTrack?.artist || 'Select a track to play'}
            </p>
            {currentTrack?.album && (
              <p className="text-bronze-500 text-sm mt-1">{currentTrack.album}</p>
            )}
          </div>

          <div className="space-y-4">
            <div>
              <input
                type="range"
                min="0"
                max={duration || 0}
                value={position}
                onChange={handleSeek}
                className="w-full h-2 bg-bronze-800 rounded-lg appearance-none cursor-pointer"
                disabled={!currentTrack}
              />
              <div className="flex justify-between text-sm text-bronze-500 mt-1">
                <span>{formatTime(position)}</span>
                <span>{formatTime(duration)}</span>
              </div>
            </div>

            <div className="flex justify-center gap-4">
              <Button
                variant="ghost"
                size="lg"
                onClick={togglePlayPause}
                disabled={!currentTrack}
              >
                {isPlaying ? (
                  <svg className="w-8 h-8" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z" clipRule="evenodd"/>
                  </svg>
                ) : (
                  <svg className="w-8 h-8" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
                  </svg>
                )}
              </Button>
            </div>
          </div>
        </Card>
      </div>
    </div>
  )
}
