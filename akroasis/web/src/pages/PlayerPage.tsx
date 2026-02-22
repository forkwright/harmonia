import { useState, useEffect, useRef } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { usePodcastStore } from '../stores/podcastStore';
import { useRadioStore } from '../stores/radioStore';
import { useEqStore } from '../stores/eqStore';
import { useCompressorStore } from '../stores/compressorStore';
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer';
import { useLyrics } from '../hooks/useLyrics';
import { Button } from '../components/Button';
import { Card } from '../components/Card';
import { AudioQualityBadges } from '../components/AudioQualityBadges';
import { LyricsDisplay } from '../components/LyricsDisplay';
import { EqualizerPanel } from '../components/EqualizerPanel';
import { SignalPath } from '../components/SignalPath';
import { WaveformSeekbar } from '../components/WaveformSeekbar';
import { getCoverArtUrl } from '../api/client';
import { isLastfmConfigured } from '../api/lastfm';
import { useArtworkViewer } from '../stores/artworkViewerStore';

export function PlayerPage() {
  const [showPipeline, setShowPipeline] = useState(false);
  const [showLyrics, setShowLyrics] = useState(false);
  const [showEq, setShowEq] = useState(false);
  const openArtwork = useArtworkViewer((s) => s.open);

  const {
    currentTrack,
    isPlaying,
    position,
    duration,
    volume,
    setVolume,
  } = usePlayerStore();

  const { currentEpisode, currentShow } = usePodcastStore();
  const isPodcast = !!currentEpisode && !!currentShow;

  const { radioMode, loading: radioLoading, stopRadio, startRadio } = useRadioStore();
  const { togglePlayPause, seek, getPipelineState, getEqualizer, getCompressor, getAnalyserNode, setCompressorParams, setCompressorEnabled: setCompressorBypass } = useWebAudioPlayer();
  const { status: lyricsStatus, lines, plainLyrics, activeLine } = useLyrics(currentTrack, position);
  const { enabled: eqEnabled, bands } = useEqStore();
  const compressor = useCompressorStore();

  const showRadioButton = isLastfmConfigured();

  const pipelineState = showPipeline ? getPipelineState() : null;

  // Sync EQ store state to the EqualizerProcessor
  const prevEnabled = useRef(eqEnabled);
  const prevBands = useRef(bands);

  useEffect(() => {
    const eq = getEqualizer();
    if (!eq) return;

    if (prevEnabled.current !== eqEnabled) {
      eq.setEnabled(eqEnabled);
      prevEnabled.current = eqEnabled;
    }

    // Always sync bands (setAllGains applies enabled state internally)
    if (eqEnabled) {
      eq.setAllGains(bands);
    }
    prevBands.current = bands;
  }, [eqEnabled, bands, getEqualizer]);

  // Sync compressor store state to the DynamicsCompressorNode
  useEffect(() => {
    const node = getCompressor();
    if (!node) return;

    if (compressor.enabled) {
      setCompressorBypass(true);
      setCompressorParams({
        threshold: compressor.threshold,
        knee: compressor.knee,
        ratio: compressor.ratio,
        attack: compressor.attack,
        release: compressor.release,
      });
    } else {
      setCompressorBypass(false);
    }
  }, [compressor.enabled, compressor.threshold, compressor.knee, compressor.ratio, compressor.attack, compressor.release, getCompressor, setCompressorParams, setCompressorBypass]);

  const handleSeek = (ms: number) => {
    seek(ms / 1000); // Convert ms to seconds
  };

  const formatTime = (ms: number) => {
    const totalSeconds = Math.floor(ms / 1000);
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, '0')}`;
  };

  const formatHz = (hz: number) => {
    if (hz >= 1000) return `${(hz / 1000).toFixed(1)}kHz`;
    return `${hz}Hz`;
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-4">
      <div className="w-full max-w-2xl">
        <Card>

          <div className="text-center mb-6">
            <div className="w-64 h-64 mx-auto mb-6 bg-bronze-800 rounded-lg flex items-center justify-center overflow-hidden">
              {isPodcast ? (
                (currentEpisode.imageUrl ?? currentShow.imageUrl) ? (
                  <img
                    src={(currentEpisode.imageUrl ?? currentShow.imageUrl)!}
                    alt={currentEpisode.title}
                    className="w-full h-full object-cover rounded-lg"
                  />
                ) : (
                  <svg className="w-24 h-24 text-bronze-600" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217z" clipRule="evenodd"/>
                  </svg>
                )
              ) : currentTrack?.coverArtUrl ? (
                <img
                  src={getCoverArtUrl(currentTrack.id, 256)}
                  alt={currentTrack.title}
                  className="w-full h-full object-cover rounded-lg cursor-zoom-in"
                  onClick={() => openArtwork(getCoverArtUrl(currentTrack.id))}
                  title="Click to view full size"
                />
              ) : (
                <svg className="w-24 h-24 text-bronze-600" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/>
                </svg>
              )}
            </div>

            {isPodcast ? (
              <>
                <h2 className="text-2xl font-bold text-bronze-100 mb-2">
                  {currentEpisode.title}
                </h2>
                <p className="text-bronze-400">
                  {currentShow.title}{currentShow.author ? ` · ${currentShow.author}` : ''}
                </p>
                {currentEpisode.description && (
                  <p className="text-bronze-500 text-sm mt-2 line-clamp-3 text-left">
                    {currentEpisode.description}
                  </p>
                )}
              </>
            ) : (
              <>
                <h2 className="text-2xl font-bold text-bronze-100 mb-2">
                  {currentTrack?.title || 'No track playing'}
                </h2>
                <p className="text-bronze-400">
                  {currentTrack?.artist || 'Select a track to play'}
                </p>
                {currentTrack?.album && (
                  <p className="text-bronze-500 text-sm mt-1">{currentTrack.album}</p>
                )}

                {currentTrack && (
                  <AudioQualityBadges
                    format={currentTrack.format}
                    sampleRate={currentTrack.sampleRate}
                    bitDepth={currentTrack.bitDepth}
                    lossless={currentTrack.format?.toLowerCase() === 'flac'}
                  />
                )}
              </>
            )}
          </div>

          <div className="space-y-4">
            <div>
              <WaveformSeekbar
                analyserNode={getAnalyserNode()}
                duration={duration}
                position={position}
                onSeek={handleSeek}
                disabled={!currentTrack}
              />
              <div className="flex justify-between text-sm text-bronze-500 mt-1">
                <span>{formatTime(position)}</span>
                <span>{formatTime(duration)}</span>
              </div>
            </div>

            <div className="flex justify-center gap-4 items-center">
              <Button
                variant="ghost"
                size="lg"
                onClick={togglePlayPause}
                disabled={!currentTrack}
                aria-label={isPlaying ? 'Pause' : 'Play'}
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

              {showRadioButton && currentTrack && !isPodcast && (
                radioMode ? (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={stopRadio}
                    aria-label="Stop radio"
                    title="Stop Radio"
                  >
                    <svg className="w-5 h-5 text-bronze-400 mr-1" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217zM14.657 2.929a1 1 0 011.414 0A9.972 9.972 0 0119 10a9.972 9.972 0 01-2.929 7.071 1 1 0 01-1.414-1.414A7.971 7.971 0 0017 10c0-2.21-.894-4.208-2.343-5.657a1 1 0 010-1.414zm-2.829 2.828a1 1 0 011.415 0A5.983 5.983 0 0115 10a5.983 5.983 0 01-1.757 4.243 1 1 0 01-1.415-1.415A3.984 3.984 0 0013 10a3.984 3.984 0 00-1.172-2.828 1 1 0 010-1.415z" clipRule="evenodd"/>
                    </svg>
                    Radio
                  </Button>
                ) : (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => startRadio(currentTrack)}
                    disabled={radioLoading}
                    aria-label="Start radio"
                    title="Start Radio from this track"
                  >
                    <svg className="w-5 h-5 mr-1" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217zM14.657 2.929a1 1 0 011.414 0A9.972 9.972 0 0119 10a9.972 9.972 0 01-2.929 7.071 1 1 0 01-1.414-1.414A7.971 7.971 0 0017 10c0-2.21-.894-4.208-2.343-5.657a1 1 0 010-1.414zm-2.829 2.828a1 1 0 011.415 0A5.983 5.983 0 0115 10a5.983 5.983 0 01-1.757 4.243 1 1 0 01-1.415-1.415A3.984 3.984 0 0013 10a3.984 3.984 0 00-1.172-2.828 1 1 0 010-1.415z" clipRule="evenodd"/>
                    </svg>
                    {radioLoading ? 'Starting···' : 'Radio'}
                  </Button>
                )
              )}
            </div>

            <div className="mt-4">
              <div className="flex items-center gap-3">
                <svg className="w-5 h-5 text-bronze-500" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217zM14.657 2.929a1 1 0 011.414 0A9.972 9.972 0 0119 10a9.972 9.972 0 01-2.929 7.071 1 1 0 01-1.414-1.414A7.971 7.971 0 0017 10c0-2.21-.894-4.208-2.343-5.657a1 1 0 010-1.414zm-2.829 2.828a1 1 0 011.415 0A5.983 5.983 0 0115 10a5.983 5.983 0 01-1.757 4.243 1 1 0 01-1.415-1.415A3.984 3.984 0 0013 10a3.984 3.984 0 00-1.172-2.828 1 1 0 010-1.415z" clipRule="evenodd"/>
                </svg>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={volume * 100}
                  onChange={(e) => setVolume(Number.parseFloat(e.target.value) / 100)}
                  className="flex-1 h-2 bg-bronze-800 rounded-lg appearance-none cursor-pointer"
                  style={{
                    backgroundImage: `linear-gradient(to right, rgb(180, 111, 63) 0%, rgb(180, 111, 63) ${volume * 100}%, rgb(37, 28, 23) ${volume * 100}%, rgb(37, 28, 23) 100%)`
                  }}
                />
                <span className="text-sm text-bronze-500 w-12 text-right">
                  {Math.round(volume * 100)}%
                </span>
              </div>
            </div>

            {/* Signal path — always visible */}
            <div className="mt-2 pt-3 border-t border-bronze-800">
              <SignalPath />
            </div>

            <div className="mt-2 space-y-3">
              {/* EQ section */}
              <div>
                <button
                  onClick={() => setShowEq(!showEq)}
                  className="text-sm text-bronze-500 hover:text-bronze-300 transition-colors flex items-center gap-1"
                >
                  <span>{showEq ? '▼' : '▶'}</span>
                  <span>Equalizer</span>
                  {!eqEnabled && (
                    <span className="ml-1 text-xs text-bronze-700">(bypassed)</span>
                  )}
                </button>

                {showEq && (
                  <div className="mt-3 border-t border-bronze-800 pt-3">
                    <EqualizerPanel />
                  </div>
                )}
              </div>

              {currentTrack && !isPodcast && (
                <>
                  <div>
                    <button
                      onClick={() => setShowLyrics(!showLyrics)}
                      className="text-sm text-bronze-500 hover:text-bronze-300 transition-colors"
                    >
                      {showLyrics ? '▼' : '▶'} Lyrics
                    </button>

                    {showLyrics && (
                      <div className="mt-2 border-t border-bronze-800 pt-3">
                        <LyricsDisplay
                          status={lyricsStatus}
                          lines={lines}
                          plainLyrics={plainLyrics}
                          activeLine={activeLine}
                        />
                      </div>
                    )}
                  </div>

                  <div>
                    <button
                      onClick={() => setShowPipeline(!showPipeline)}
                      className="text-sm text-bronze-500 hover:text-bronze-300 transition-colors"
                    >
                      {showPipeline ? '▼' : '▶'} Pipeline Details
                    </button>

                    {showPipeline && pipelineState && (
                      <div className="mt-2 p-3 bg-bronze-900/50 rounded-lg text-xs space-y-2">
                        <div className="flex items-center justify-between">
                          <span className="text-bronze-500">Input:</span>
                          <span className="text-bronze-300">
                            {pipelineState.inputFormat.codec.toUpperCase()} • {formatHz(pipelineState.inputFormat.sampleRate)} • {pipelineState.inputFormat.bitDepth}-bit • {pipelineState.inputFormat.channels}ch
                          </span>
                        </div>
                        <div className="flex items-center justify-between">
                          <span className="text-bronze-500">Output:</span>
                          <span className="text-bronze-300">
                            {formatHz(pipelineState.outputDevice.sampleRate)} • {pipelineState.outputDevice.channels}ch
                          </span>
                        </div>
                        <div className="flex items-center justify-between">
                          <span className="text-bronze-500">Latency:</span>
                          <span className="text-bronze-300">
                            {(pipelineState.latency * 1000).toFixed(1)}ms
                          </span>
                        </div>
                        <div className="flex items-center justify-between">
                          <span className="text-bronze-500">Buffer:</span>
                          <span className="text-bronze-300">
                            {(pipelineState.bufferSize / pipelineState.outputDevice.sampleRate).toFixed(2)}s
                          </span>
                        </div>
                      </div>
                    )}
                  </div>
                </>
              )}
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}
