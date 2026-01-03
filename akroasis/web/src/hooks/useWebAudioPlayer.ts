// React hook for Web Audio API player integration
import { useEffect, useRef, useCallback } from 'react';
import { WebAudioPlayer } from '../audio/WebAudioPlayer';
import { usePlayerStore } from '../stores/playerStore';
import { getStreamUrl } from '../api/client';

export function useWebAudioPlayer() {
  const playerRef = useRef<WebAudioPlayer | null>(null);
  const intervalRef = useRef<number | null>(null);

  const {
    currentTrack,
    isPlaying,
    volume,
    queue,
    setIsPlaying,
    setPosition,
    setDuration
  } = usePlayerStore();

  // Initialize player
  useEffect(() => {
    playerRef.current = new WebAudioPlayer();

    playerRef.current.setPlaybackEndCallback(() => {
      // Track ended - move to next in queue
      const { queue: currentQueue, currentTrack: current, setCurrentTrack, setIsPlaying } = usePlayerStore.getState();
      const currentIndex = currentQueue.findIndex(t => t.id === current?.id);
      if (currentIndex !== -1 && currentIndex < currentQueue.length - 1) {
        const nextTrack = currentQueue[currentIndex + 1];
        setCurrentTrack(nextTrack);
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
  }, []);

  // Update volume when store changes
  useEffect(() => {
    if (playerRef.current) {
      playerRef.current.setVolume(volume);
    }
  }, [volume]);

  // Position tracking interval
  useEffect(() => {
    if (isPlaying && playerRef.current) {
      intervalRef.current = window.setInterval(() => {
        const currentTime = playerRef.current?.getCurrentTime() ?? 0;
        setPosition(currentTime * 1000); // Convert to ms for store
      }, 100);
    } else {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
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

    const currentIndex = queue.findIndex(t => t.id === currentTrack.id);
    if (currentIndex !== -1 && currentIndex < queue.length - 1) {
      const nextTrack = queue[currentIndex + 1];
      const nextStreamUrl = getStreamUrl(nextTrack.id);
      playerRef.current.preloadNext(nextTrack, nextStreamUrl);
    }
  }, [currentTrack, queue]);

  const playTrack = useCallback(async (track: typeof currentTrack) => {
    if (!track || !playerRef.current) return;

    try {
      const streamUrl = getStreamUrl(track.id);
      await playerRef.current.loadTrack(track, streamUrl);

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

  return {
    playTrack,
    togglePlayPause,
    seek,
    getPipelineState
  };
}
