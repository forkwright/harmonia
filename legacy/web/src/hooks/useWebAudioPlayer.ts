// React hook for audio player integration
// Pattern: single persistent audio element, event-driven (not polling)
// Modeled after Jellyfin/Navidrome approach
import { useEffect, useRef, useCallback } from 'react';
import { getPlayer } from '../audio/playerSingleton';
import { usePlayerStore } from '../stores/playerStore';
import { useRadioStore } from '../stores/radioStore';
import { getStreamUrl } from '../api/client';

export function useWebAudioPlayer() {
  const playerRef = useRef(getPlayer()); // Always the same singleton

  // Read from store once for initial setup; use getState() in callbacks to avoid stale closures
  const setIsPlaying = usePlayerStore((s) => s.setIsPlaying);
  const setPosition = usePlayerStore((s) => s.setPosition);
  const setDuration = usePlayerStore((s) => s.setDuration);

  // Wire up callbacks ONCE (singleton means this is idempotent)
  const callbacksSet = useRef(false);
  useEffect(() => {
    if (callbacksSet.current) return;
    callbacksSet.current = true;
    const player = playerRef.current;

    player.setPlaybackEndCallback(() => {
      const store = usePlayerStore.getState();
      const { queue: q, currentTrack: cur, repeatMode } = store;

      if (repeatMode === 'one') {
        player.replay();
        return;
      }

      const idx = q.findIndex(t => t.id === cur?.id);
      const hasNext = idx !== -1 && idx < q.length - 1;

      useRadioStore.getState().replenishIfNeeded(hasNext ? q[idx + 1] : cur);

      if (hasNext) {
        store.setCurrentTrack(q[idx + 1]);
      } else if (repeatMode === 'all' && q.length > 0) {
        store.setCurrentTrack(q[0]);
      } else if (repeatMode === 'shuffle-repeat' && q.length > 0) {
        const shuffled = [...q].sort(() => Math.random() - 0.5);
        store.setQueue(shuffled);
        store.setCurrentTrack(shuffled[0]);
      } else {
        store.setIsPlaying(false);
      }
    });

    player.setPlaybackErrorCallback((error) => {
      console.error('Playback error:', error);
      usePlayerStore.getState().setIsPlaying(false);
    });

    // No cleanup — singleton lives for the app lifetime
  }, []); // Empty deps — runs once

  // Sync volume to player
  const volume = usePlayerStore((s) => s.volume);
  useEffect(() => {
    playerRef.current?.setVolume(volume);
  }, [volume]);

  // Sync playback speed
  const playbackSpeed = usePlayerStore((s) => s.playbackSpeed);
  useEffect(() => {
    playerRef.current?.setPlaybackSpeed(playbackSpeed);
  }, [playbackSpeed]);

  // Position tracking — use requestAnimationFrame for smooth updates, only when playing
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    if (!isPlaying) {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
      return;
    }

    let lastUpdate = 0;
    const tick = (now: number) => {
      if (!playerRef.current) return;
      // Throttle to ~4fps (250ms) to reduce store updates
      if (now - lastUpdate > 250) {
        lastUpdate = now;
        const t = playerRef.current.getCurrentTime();
        const d = playerRef.current.getDuration();
        setPosition(t * 1000);
        if (d > 0) setDuration(d * 1000);
      }
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);

    return () => {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
    };
  }, [isPlaying, setPosition, setDuration]);

  // Preload next track
  const currentTrack = usePlayerStore((s) => s.currentTrack);
  const queue = usePlayerStore((s) => s.queue);

  useEffect(() => {
    if (!currentTrack || !playerRef.current) return;
    const { repeatMode } = usePlayerStore.getState();
    const idx = queue.findIndex(t => t.id === currentTrack.id);

    if (idx !== -1 && idx < queue.length - 1) {
      const next = queue[idx + 1];
      playerRef.current.preloadNext(next, getStreamUrl(next.id));
    } else if (idx === queue.length - 1 && (repeatMode === 'all' || repeatMode === 'shuffle-repeat') && queue.length > 0) {
      playerRef.current.preloadNext(queue[0], getStreamUrl(queue[0].id));
    }
  }, [currentTrack, queue]);

  // Play track — the core action
  const playTrack = useCallback(async (track: typeof currentTrack) => {
    if (!track || !playerRef.current) return;
    try {
      await playerRef.current.loadTrack(track, getStreamUrl(track.id));
      const d = playerRef.current.getDuration();
      if (d > 0) setDuration(d * 1000);
      setIsPlaying(true);
    } catch (error) {
      console.error('Failed to play track:', error);
      setIsPlaying(false);
    }
  }, [setDuration, setIsPlaying]);

  // Auto-play on track change
  useEffect(() => {
    if (currentTrack && usePlayerStore.getState().isPlaying) {
      playTrack(currentTrack);
    }
  }, [currentTrack?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // Toggle play/pause — read isPlaying from store directly, never stale
  const togglePlayPause = useCallback(() => {
    const player = playerRef.current;
    if (!player) return;

    const playing = usePlayerStore.getState().isPlaying;
    if (playing) {
      player.pause();
      setIsPlaying(false);
    } else {
      player.play();
      setIsPlaying(true);
    }
  }, [setIsPlaying]);

  const seek = useCallback((timeSeconds: number) => {
    playerRef.current?.seek(timeSeconds);
    setPosition(timeSeconds * 1000);
  }, [setPosition]);

  // Player info accessors
  const getPlaybackInfo = useCallback(() => playerRef.current?.getPlaybackInfo() ?? null, []);
  const getPipelineState = useCallback(() => playerRef.current?.getPipelineState() ?? null, []);
  const getEqualizer = useCallback(() => playerRef.current?.getEqualizer() ?? null, []);
  const getCompressor = useCallback(() => playerRef.current?.getCompressor() ?? null, []);
  const getAnalyserNode = useCallback(() => playerRef.current?.getAnalyserNode() ?? null, []);
  const setCompressorParams = useCallback((p: Record<string, number>) => playerRef.current?.setCompressorParams(p), []);
  const setCompressorEnabled = useCallback((e: boolean) => playerRef.current?.setCompressorEnabled(e), []);

  return {
    playTrack,
    togglePlayPause,
    seek,
    getPlaybackInfo,
    getPipelineState,
    getEqualizer,
    getCompressor,
    getAnalyserNode,
    setCompressorParams,
    setCompressorEnabled,
  };
}
