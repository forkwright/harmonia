// Media Session API for media keys and lock screen controls
import { useEffect } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer';
import { getCoverArtUrl } from '../api/client';

export function useMediaSession() {
  const { currentTrack, isPlaying, queue } = usePlayerStore();
  const { togglePlayPause, seek } = useWebAudioPlayer();

  useEffect(() => {
    if (!('mediaSession' in navigator)) {
      console.warn('Media Session API not supported');
      return;
    }

    // Update metadata when track changes
    if (currentTrack) {
      navigator.mediaSession.metadata = new MediaMetadata({
        title: currentTrack.title,
        artist: currentTrack.artist,
        album: currentTrack.album || undefined,
        artwork: [
          {
            src: getCoverArtUrl(currentTrack.id, 96),
            sizes: '96x96',
            type: 'image/jpeg',
          },
          {
            src: getCoverArtUrl(currentTrack.id, 256),
            sizes: '256x256',
            type: 'image/jpeg',
          },
          {
            src: getCoverArtUrl(currentTrack.id, 512),
            sizes: '512x512',
            type: 'image/jpeg',
          },
        ],
      });

      // Update playback state
      navigator.mediaSession.playbackState = isPlaying ? 'playing' : 'paused';
    } else {
      navigator.mediaSession.metadata = null;
      navigator.mediaSession.playbackState = 'none';
    }

    // Setup action handlers
    navigator.mediaSession.setActionHandler('play', () => {
      if (!isPlaying) {
        togglePlayPause();
      }
    });

    navigator.mediaSession.setActionHandler('pause', () => {
      if (isPlaying) {
        togglePlayPause();
      }
    });

    navigator.mediaSession.setActionHandler('stop', () => {
      if (isPlaying) {
        togglePlayPause();
      }
    });

    navigator.mediaSession.setActionHandler('seekbackward', (details) => {
      const skipTime = details.seekOffset || 10;
      const currentPos = usePlayerStore.getState().position / 1000;
      seek(Math.max(0, currentPos - skipTime));
    });

    navigator.mediaSession.setActionHandler('seekforward', (details) => {
      const skipTime = details.seekOffset || 10;
      const currentPos = usePlayerStore.getState().position / 1000;
      seek(currentPos + skipTime);
    });

    navigator.mediaSession.setActionHandler('previoustrack', () => {
      if (currentTrack && queue.length > 0) {
        const currentIndex = queue.findIndex((t) => t.id === currentTrack.id);
        if (currentIndex > 0) {
          const prevTrack = queue[currentIndex - 1];
          usePlayerStore.getState().setCurrentTrack(prevTrack);
        }
      }
    });

    navigator.mediaSession.setActionHandler('nexttrack', () => {
      if (currentTrack && queue.length > 0) {
        const currentIndex = queue.findIndex((t) => t.id === currentTrack.id);
        if (currentIndex !== -1 && currentIndex < queue.length - 1) {
          const nextTrack = queue[currentIndex + 1];
          usePlayerStore.getState().setCurrentTrack(nextTrack);
        }
      }
    });

    // Update position state periodically
    const updatePositionState = () => {
      if (currentTrack && isPlaying) {
        const position = usePlayerStore.getState().position / 1000; // Convert to seconds
        const duration = usePlayerStore.getState().duration / 1000;

        if (duration > 0 && 'setPositionState' in navigator.mediaSession) {
          try {
            navigator.mediaSession.setPositionState({
              duration: duration,
              playbackRate: 1,
              position: position,
            });
          } catch (error) {
            console.warn('Failed to update position state:', error);
          }
        }
      }
    };

    const interval = setInterval(updatePositionState, 1000);

    return () => {
      clearInterval(interval);
      // Clear action handlers on cleanup
      if ('mediaSession' in navigator) {
        navigator.mediaSession.setActionHandler('play', null);
        navigator.mediaSession.setActionHandler('pause', null);
        navigator.mediaSession.setActionHandler('stop', null);
        navigator.mediaSession.setActionHandler('seekbackward', null);
        navigator.mediaSession.setActionHandler('seekforward', null);
        navigator.mediaSession.setActionHandler('previoustrack', null);
        navigator.mediaSession.setActionHandler('nexttrack', null);
      }
    };
  }, [currentTrack, isPlaying, queue, togglePlayPause, seek]);
}
