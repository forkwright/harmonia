// React hook for Web Audio API player integration
import { useEffect, useRef, useCallback } from 'react';
import { WebAudioPlayer } from '../audio/WebAudioPlayer';
import { usePlayerStore } from '../stores/playerStore';
import { useRadioStore } from '../stores/radioStore';
import { useReplayGainStore } from '../stores/replayGainStore';
import { useMetaxisStore } from '../stores/metaxisStore';
import { getStreamUrl } from '../api/client';

export function useWebAudioPlayer() {
  const playerRef = useRef<WebAudioPlayer | null>(null);
  const intervalRef = useRef<number | null>(null);

  const {
    currentTrack,
    isPlaying,
    volume,
    playbackSpeed,
    queue,
    setIsPlaying,
    setPosition,
    setDuration
  } = usePlayerStore();

  // Initialize player
  useEffect(() => {
    playerRef.current = new WebAudioPlayer();

    playerRef.current.setPlaybackEndCallback(() => {
      const { queue: currentQueue, currentTrack: current, repeatMode, setCurrentTrack, setIsPlaying, setQueue } = usePlayerStore.getState();

      // Repeat one: replay immediately
      if (repeatMode === 'one') {
        playerRef.current?.replay();
        return;
      }

      const currentIndex = currentQueue.findIndex(t => t.id === current?.id);
      const hasNext = currentIndex !== -1 && currentIndex < currentQueue.length - 1;

      // Trigger radio replenishment if active (fires async, non-blocking)
      const nextTrack = hasNext ? currentQueue[currentIndex + 1] : null;
      useRadioStore.getState().replenishIfNeeded(nextTrack ?? current);

      if (hasNext) {
        setCurrentTrack(currentQueue[currentIndex + 1]);
      } else if (repeatMode === 'all') {
        if (currentQueue.length > 0) {
          setCurrentTrack(currentQueue[0]);
        }
      } else if (repeatMode === 'shuffle-repeat') {
        if (currentQueue.length > 0) {
          const reshuffled = [...currentQueue].sort(() => Math.random() - 0.5);
          setQueue(reshuffled);
          setCurrentTrack(reshuffled[0]);
        }
      } else if (useRadioStore.getState().radioMode) {
        setIsPlaying(false);
      } else {
        setIsPlaying(false);
      }
    });

    playerRef.current.setPlaybackErrorCallback((error) => {
      console.error('Playback error:', error);
      setIsPlaying(false);
    });

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
      playerRef.current?.close();
    };
  }, [setIsPlaying]);

  // Update volume when store changes
  useEffect(() => {
    if (playerRef.current) {
      playerRef.current.setVolume(volume);
    }
  }, [volume]);

  // Update playback speed when store changes
  useEffect(() => {
    if (playerRef.current) {
      playerRef.current.setPlaybackSpeed(playbackSpeed);
    }
  }, [playbackSpeed]);

  // Position tracking interval + crossfade trigger
  const crossfadeTriggeredRef = useRef(false);

  useEffect(() => {
    crossfadeTriggeredRef.current = false;
  }, [currentTrack?.id]);

  useEffect(() => {
    if (isPlaying && playerRef.current) {
      intervalRef.current = globalThis.setInterval(() => {
        if (!playerRef.current) return;
        const currentTime = playerRef.current.getCurrentTime();
        setPosition(currentTime * 1000); // Convert to ms for store

        // Crossfade trigger: check if we're within crossfade duration of track end
        if (!crossfadeTriggeredRef.current && !playerRef.current.getIsCrossfading()) {
          const { mode, duration: cfDuration, curve, shouldCrossfade } = useMetaxisStore.getState();
          if (mode !== 'off' && cfDuration > 0) {
            const trackDuration = playerRef.current.getDuration();
            const remaining = trackDuration - currentTime;
            const { repeatMode } = usePlayerStore.getState();

            // Don't crossfade for repeat-one
            if (remaining <= cfDuration && remaining > 0 && trackDuration > cfDuration * 2 && repeatMode !== 'one') {
              const { queue: currentQueue, currentTrack: current } = usePlayerStore.getState();
              const currentIndex = currentQueue.findIndex(t => t.id === current?.id);
              const nextTrack = currentIndex !== -1 && currentIndex < currentQueue.length - 1
                ? currentQueue[currentIndex + 1]
                : null;

              if (nextTrack && shouldCrossfade(current?.album, nextTrack.album)) {
                crossfadeTriggeredRef.current = true;
                // Fetch next track buffer and start crossfade
                const nextUrl = getStreamUrl(nextTrack.id);
                fetch(nextUrl)
                  .then(r => r.arrayBuffer())
                  .then(ab => {
                    if (!playerRef.current) return;
                    const ctx = playerRef.current?.getAudioContext();
                    if (!ctx) return;
                    return ctx.decodeAudioData(ab);
                  })
                  .then(buffer => {
                    if (buffer && playerRef.current) {
                      playerRef.current.startCrossfade(buffer, nextTrack, remaining, curve);
                    }
                  })
                  .catch(() => {
                    // Crossfade failed, fall back to gapless
                    crossfadeTriggeredRef.current = false;
                  });
              }
            }
          }
        }
      }, 100);
    }

    if (!isPlaying && intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [isPlaying, setPosition]);

  // Preload next track when queue changes
  useEffect(() => {
    if (!currentTrack || !playerRef.current) return;

    const { repeatMode } = usePlayerStore.getState();
    const currentIndex = queue.findIndex(t => t.id === currentTrack.id);

    if (currentIndex !== -1 && currentIndex < queue.length - 1) {
      const nextTrack = queue[currentIndex + 1];
      const nextStreamUrl = getStreamUrl(nextTrack.id);
      playerRef.current.preloadNext(nextTrack, nextStreamUrl);
    } else if (currentIndex === queue.length - 1 && (repeatMode === 'all' || repeatMode === 'shuffle-repeat') && queue.length > 0) {
      // At end of queue with repeat — preload first track for seamless wrap
      const firstTrack = queue[0];
      const firstStreamUrl = getStreamUrl(firstTrack.id);
      playerRef.current.preloadNext(firstTrack, firstStreamUrl);
    }
  }, [currentTrack, queue]);

  const playTrack = useCallback(async (track: typeof currentTrack) => {
    if (!track || !playerRef.current) return;

    try {
      const streamUrl = getStreamUrl(track.id);
      await playerRef.current.loadTrack(track, streamUrl);

      // Apply ReplayGain
      const { mode: rgMode, getEffectiveGain, preScanEnabled, limiterEnabled } = useReplayGainStore.getState();
      if (rgMode !== 'off') {
        const gainDb = getEffectiveGain(track);
        // If no tags/cache and preScan enabled, analyze the buffer
        if (gainDb === null && preScanEnabled) {
          const pipelineState = playerRef.current.getPipelineState();
          if (pipelineState) {
            // Buffer is already decoded in the player — we'd need access. For now, cache result on future loads.
            // Real-time scan happens when buffer is available via preload path.
          }
        }
        playerRef.current.setReplayGain(gainDb);
        playerRef.current.setLimiterEnabled(limiterEnabled);
      } else {
        playerRef.current.setReplayGain(null);
      }

      const duration = playerRef.current.getDuration();
      setDuration(duration * 1000); // Convert to ms for store
      setIsPlaying(true);
    } catch (error) {
      console.error('Failed to play track:', error);
      setIsPlaying(false);
    }
  }, [setDuration, setIsPlaying]);

  // Auto-play when currentTrack changes (from queue or library)
  useEffect(() => {
    if (currentTrack && isPlaying) {
      playTrack(currentTrack);
    }
    // Note: Intentionally only depend on currentTrack?.id to trigger this effect only when track changes.
    // Including isPlaying would cause track reload on every play/pause toggle.
    // Including playTrack (Zustand store setter) is unnecessary - setters are stable references.
  }, [currentTrack?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const togglePlayPause = useCallback(() => {
    if (!playerRef.current) return;

    if (isPlaying) {
      playerRef.current.pause();
      setIsPlaying(false);
    } else {
      playerRef.current.play();
      setIsPlaying(true);
    }
  }, [isPlaying, setIsPlaying]);

  const seek = useCallback((timeSeconds: number) => {
    if (!playerRef.current) return;

    playerRef.current.seek(timeSeconds);
    setPosition(timeSeconds * 1000);
  }, [setPosition]);

  const getPipelineState = useCallback(() => {
    return playerRef.current?.getPipelineState() ?? null;
  }, []);

  const getEqualizer = useCallback(() => {
    return playerRef.current?.getEqualizer() ?? null;
  }, []);

  const getCompressor = useCallback(() => {
    return playerRef.current?.getCompressor() ?? null;
  }, []);

  const getAnalyserNode = useCallback(() => {
    return playerRef.current?.getAnalyserNode() ?? null;
  }, []);

  const setCompressorParams = useCallback((params: {
    threshold?: number;
    knee?: number;
    ratio?: number;
    attack?: number;
    release?: number;
  }) => {
    playerRef.current?.setCompressorParams(params);
  }, []);

  const setCompressorEnabled = useCallback((enabled: boolean) => {
    playerRef.current?.setCompressorEnabled(enabled);
  }, []);

  return {
    playTrack,
    togglePlayPause,
    seek,
    getPipelineState,
    getEqualizer,
    getCompressor,
    getAnalyserNode,
    setCompressorParams,
    setCompressorEnabled,
  };
}
