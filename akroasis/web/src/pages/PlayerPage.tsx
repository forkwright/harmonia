import { useState, useEffect, useRef } from 'react'
import { usePlayerStore } from '../stores/playerStore'
import { usePodcastStore } from '../stores/podcastStore'
import { useRadioStore } from '../stores/radioStore'
import { useEqStore } from '../stores/eqStore'
import { useCompressorStore } from '../stores/compressorStore'
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer'
import { useLyrics } from '../hooks/useLyrics'
import { AudioQualityBadges } from '../components/AudioQualityBadges'
import { LyricsDisplay } from '../components/LyricsDisplay'
import { EqualizerPanel } from '../components/EqualizerPanel'
import { SignalPath } from '../components/SignalPath'
import { RepeatButton } from '../components/RepeatButton'
import { HeartButton } from '../components/HeartButton'
import { WaveformSeekbar } from '../components/WaveformSeekbar'
import { getCoverArtUrl } from '../api/client'
import { isLastfmConfigured } from '../api/lastfm'
import { useArtworkViewer } from '../stores/artworkViewerStore'

function formatTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  return `${minutes}:${seconds.toString().padStart(2, '0')}`
}

function formatHz(hz: number): string {
  if (hz >= 1000) return `${(hz / 1000).toFixed(1)}kHz`
  return `${hz}Hz`
}

interface ExpandableSectionProps {
  label: string
  badge?: string
  children: React.ReactNode
  defaultOpen?: boolean
}

function ExpandableSection({ label, badge, children, defaultOpen = false }: ExpandableSectionProps) {
  const [open, setOpen] = useState(defaultOpen)

  return (
    <div className="border-t border-bronze-800/50">
      <button
        onClick={() => setOpen(!open)}
        className="w-full flex items-center gap-2 py-3 text-sm text-bronze-400 hover:text-bronze-200 transition-colors"
      >
        <svg
          className={`w-3.5 h-3.5 transition-transform ${open ? 'rotate-90' : ''}`}
          fill="currentColor"
          viewBox="0 0 20 20"
        >
          <path fillRule="evenodd" d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z" clipRule="evenodd" />
        </svg>
        <span>{label}</span>
        {badge && (
          <span className="text-xs text-bronze-600">{badge}</span>
        )}
      </button>
      {open && (
        <div className="pb-4 animate-[fadeIn_150ms_ease-out]">
          {children}
        </div>
      )}
    </div>
  )
}

export function PlayerPage() {
  const openArtwork = useArtworkViewer((s) => s.open)

  const {
    currentTrack,
    isPlaying,
    position,
    duration,
    volume,
    setVolume,
  } = usePlayerStore()

  const { currentEpisode, currentShow } = usePodcastStore()
  const isPodcast = !!currentEpisode && !!currentShow

  const { radioMode, loading: radioLoading, stopRadio, startRadio } = useRadioStore()
  const { togglePlayPause, seek, getPipelineState, getEqualizer, getCompressor, getAnalyserNode, setCompressorParams, setCompressorEnabled: setCompressorBypass } = useWebAudioPlayer()
  const { status: lyricsStatus, lines, plainLyrics, activeLine } = useLyrics(currentTrack, position)
  const { enabled: eqEnabled, bands } = useEqStore()
  const compressor = useCompressorStore()

  const showRadioButton = isLastfmConfigured()

  // Sync EQ store state to the EqualizerProcessor
  const prevEnabled = useRef(eqEnabled)
  const prevBands = useRef(bands)

  useEffect(() => {
    const eq = getEqualizer()
    if (!eq) return

    if (prevEnabled.current !== eqEnabled) {
      eq.setEnabled(eqEnabled)
      prevEnabled.current = eqEnabled
    }

    if (eqEnabled) {
      eq.setAllGains(bands)
    }
    prevBands.current = bands
  }, [eqEnabled, bands, getEqualizer])

  // Sync compressor store state
  useEffect(() => {
    const node = getCompressor()
    if (!node) return

    if (compressor.enabled) {
      setCompressorBypass(true)
      setCompressorParams({
        threshold: compressor.threshold,
        knee: compressor.knee,
        ratio: compressor.ratio,
        attack: compressor.attack,
        release: compressor.release,
      })
    } else {
      setCompressorBypass(false)
    }
  }, [compressor.enabled, compressor.threshold, compressor.knee, compressor.ratio, compressor.attack, compressor.release, getCompressor, setCompressorParams, setCompressorBypass])

  const handleSeek = (ms: number) => {
    seek(ms / 1000)
  }

  // Empty state
  if (!currentTrack && !isPodcast) {
    return (
      <div className="min-h-[calc(100vh-4rem)] flex flex-col items-center justify-center p-8">
        <div className="w-32 h-32 rounded-2xl bg-bronze-900 flex items-center justify-center mb-6">
          <svg className="w-16 h-16 text-bronze-700" fill="currentColor" viewBox="0 0 20 20">
            <path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/>
          </svg>
        </div>
        <p className="text-bronze-400 text-lg">Nothing playing</p>
        <p className="text-bronze-600 text-sm mt-1">Pick something from the library</p>
      </div>
    )
  }

  const title = isPodcast ? currentEpisode.title : currentTrack?.title ?? ''
  const artist = isPodcast
    ? `${currentShow.title}${currentShow.author ? ` · ${currentShow.author}` : ''}`
    : currentTrack?.artist ?? ''
  const album = isPodcast ? undefined : currentTrack?.album

  const coverUrl = isPodcast
    ? (currentEpisode.imageUrl ?? currentShow.imageUrl)
    : currentTrack?.coverArtUrl
      ? getCoverArtUrl(currentTrack.id, 512)
      : null

  return (
    <div className="min-h-[calc(100vh-4rem)] flex items-start justify-center px-4 py-8">
      <div className="w-full max-w-lg">

        {/* Album Art */}
        <div className="mb-8">
          <div
            className="w-full aspect-square rounded-2xl overflow-hidden bg-bronze-900 shadow-2xl shadow-black/30"
            role={coverUrl ? 'button' : undefined}
            onClick={coverUrl ? () => openArtwork(isPodcast ? coverUrl! : getCoverArtUrl(currentTrack!.id)) : undefined}
            style={coverUrl ? { cursor: 'zoom-in' } : undefined}
          >
            {coverUrl ? (
              <img
                src={coverUrl}
                alt={title}
                className="w-full h-full object-cover"
              />
            ) : (
              <div className="w-full h-full flex items-center justify-center">
                <svg className="w-24 h-24 text-bronze-700" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/>
                </svg>
              </div>
            )}
          </div>
        </div>

        {/* Track info */}
        <div className="mb-6">
          <h1 className="text-2xl font-bold text-bronze-50 leading-tight">{title}</h1>
          <p className="text-bronze-400 mt-1">{artist}</p>
          {album && <p className="text-bronze-500 text-sm mt-0.5">{album}</p>}

          {isPodcast && currentEpisode.description && (
            <p className="text-bronze-500 text-sm mt-3 line-clamp-2">{currentEpisode.description}</p>
          )}

          {currentTrack && !isPodcast && (
            <AudioQualityBadges
              format={currentTrack.format}
              sampleRate={currentTrack.sampleRate}
              bitDepth={currentTrack.bitDepth}
              lossless={currentTrack.format?.toLowerCase() === 'flac'}
            />
          )}
        </div>

        {/* Seekbar + time */}
        <div className="mb-6">
          <WaveformSeekbar
            analyserNode={getAnalyserNode()}
            duration={duration}
            position={position}
            onSeek={handleSeek}
            disabled={!currentTrack && !isPodcast}
          />
          <div className="flex justify-between text-xs text-bronze-500 mt-1.5 tabular-nums">
            <span>{formatTime(position)}</span>
            <span>{formatTime(duration)}</span>
          </div>
        </div>

        {/* Transport controls */}
        <div className="flex items-center justify-center gap-6 mb-6">
          {showRadioButton && currentTrack && !isPodcast && (
            <button
              onClick={radioMode ? stopRadio : () => startRadio(currentTrack)}
              disabled={radioLoading}
              className={`p-2 rounded-full transition-colors ${
                radioMode
                  ? 'text-bronze-200 bg-bronze-700'
                  : 'text-bronze-500 hover:text-bronze-300'
              }`}
              title={radioMode ? 'Stop Radio' : 'Start Radio'}
              aria-label={radioMode ? 'Stop radio' : 'Start radio'}
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217zM14.657 2.929a1 1 0 011.414 0A9.972 9.972 0 0119 10a9.972 9.972 0 01-2.929 7.071 1 1 0 01-1.414-1.414A7.971 7.971 0 0017 10c0-2.21-.894-4.208-2.343-5.657a1 1 0 010-1.414zm-2.829 2.828a1 1 0 011.415 0A5.983 5.983 0 0115 10a5.983 5.983 0 01-1.757 4.243 1 1 0 01-1.415-1.415A3.984 3.984 0 0013 10a3.984 3.984 0 00-1.172-2.828 1 1 0 010-1.415z" clipRule="evenodd"/>
              </svg>
            </button>
          )}

          <button
            onClick={togglePlayPause}
            disabled={!currentTrack && !isPodcast}
            className="w-16 h-16 flex items-center justify-center rounded-full bg-bronze-100 text-bronze-950 hover:bg-white transition-colors disabled:opacity-40"
            aria-label={isPlaying ? 'Pause' : 'Play'}
          >
            {isPlaying ? (
              <svg className="w-7 h-7" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z" clipRule="evenodd"/>
              </svg>
            ) : (
              <svg className="w-7 h-7 ml-0.5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
              </svg>
            )}
          </button>

          <RepeatButton />
        </div>

        {/* Favorites */}
        {currentTrack && (
          <div className="flex justify-center mb-4">
            <HeartButton trackId={currentTrack.id} />
          </div>
        )}

        {/* Volume */}
        <div className="flex items-center gap-3 mb-8">
          <svg className="w-4 h-4 text-bronze-600 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
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
          <span className="text-xs text-bronze-500 w-10 text-right tabular-nums">
            {Math.round(volume * 100)}%
          </span>
        </div>

        {/* Signal path — always visible */}
        <div className="mb-2">
          <SignalPath />
        </div>

        {/* Expandable sections */}
        <div>
          <ExpandableSection label="Equalizer" badge={eqEnabled ? undefined : '(bypassed)'}>
            <EqualizerPanel />
          </ExpandableSection>

          {currentTrack && !isPodcast && (
            <>
              <ExpandableSection label="Lyrics">
                <LyricsDisplay
                  status={lyricsStatus}
                  lines={lines}
                  plainLyrics={plainLyrics}
                  activeLine={activeLine}
                />
              </ExpandableSection>

              <ExpandableSection label="Pipeline Details">
                {(() => {
                  // Force re-evaluate pipeline state when section opens
                  const ps = getPipelineState()
                  if (!ps) return <p className="text-sm text-bronze-500">No pipeline data available</p>
                  return (
                    <div className="p-3 bg-bronze-900/50 rounded-lg text-xs space-y-2">
                      <div className="flex items-center justify-between">
                        <span className="text-bronze-500">Input:</span>
                        <span className="text-bronze-300">
                          {ps.inputFormat.codec.toUpperCase()} • {formatHz(ps.inputFormat.sampleRate)} • {ps.inputFormat.bitDepth}-bit • {ps.inputFormat.channels}ch
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-bronze-500">Output:</span>
                        <span className="text-bronze-300">
                          {formatHz(ps.outputDevice.sampleRate)} • {ps.outputDevice.channels}ch
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-bronze-500">Latency:</span>
                        <span className="text-bronze-300">
                          {(ps.latency * 1000).toFixed(1)}ms
                        </span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-bronze-500">Buffer:</span>
                        <span className="text-bronze-300">
                          {(ps.bufferSize / ps.outputDevice.sampleRate).toFixed(2)}s
                        </span>
                      </div>
                    </div>
                  )
                })()}
              </ExpandableSection>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
