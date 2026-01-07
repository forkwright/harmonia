// Global keyboard shortcuts for media control
import { useEffect } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer';
import { useNavigate, useLocation } from 'react-router-dom';

type KeyHandler = (event: KeyboardEvent) => void;

export function useKeyboardShortcuts() {
  const { currentTrack, queue, isPlaying } = usePlayerStore();
  const { togglePlayPause, seek } = useWebAudioPlayer();
  const navigate = useNavigate();
  const location = useLocation();

  useEffect(() => {
    const handlePlayPause: KeyHandler = (event) => {
      event.preventDefault();
      togglePlayPause();
    };

    const handleSeekForward: KeyHandler = (event) => {
      event.preventDefault();
      if (currentTrack) {
        const currentPos = usePlayerStore.getState().position / 1000;
        seek(currentPos + 10);
      }
    };

    const handleSeekBackward: KeyHandler = (event) => {
      event.preventDefault();
      if (currentTrack) {
        const currentPos = usePlayerStore.getState().position / 1000;
        seek(Math.max(0, currentPos - 10));
      }
    };

    const handleVolumeUp: KeyHandler = (event) => {
      event.preventDefault();
      const currentVolume = usePlayerStore.getState().volume;
      usePlayerStore.getState().setVolume(Math.min(1, currentVolume + 0.1));
    };

    const handleVolumeDown: KeyHandler = (event) => {
      event.preventDefault();
      const currentVolume = usePlayerStore.getState().volume;
      usePlayerStore.getState().setVolume(Math.max(0, currentVolume - 0.1));
    };

    const handleNextTrack: KeyHandler = (event) => {
      event.preventDefault();
      if (currentTrack && queue.length > 0) {
        const currentIndex = queue.findIndex((t) => t.id === currentTrack.id);
        if (currentIndex !== -1 && currentIndex < queue.length - 1) {
          const nextTrack = queue[currentIndex + 1];
          usePlayerStore.getState().setCurrentTrack(nextTrack);
        }
      }
    };

    const handlePreviousTrack: KeyHandler = (event) => {
      event.preventDefault();
      if (currentTrack && queue.length > 0) {
        const currentIndex = queue.findIndex((t) => t.id === currentTrack.id);
        if (currentIndex > 0) {
          const prevTrack = queue[currentIndex - 1];
          usePlayerStore.getState().setCurrentTrack(prevTrack);
        }
      }
    };

    const handleNavigateTo = (path: string): KeyHandler => (event) => {
      if (!event.ctrlKey && !event.metaKey) {
        event.preventDefault();
        navigate(path);
      }
    };

    const handleShowHelp: KeyHandler = (event) => {
      event.preventDefault();
      console.log('Keyboard Shortcuts:', {
        'Space/K': 'Play/Pause',
        'Right/L': 'Seek +10s',
        'Left/J': 'Seek -10s',
        'Up': 'Volume +10%',
        'Down': 'Volume -10%',
        'N': 'Next track',
        'P': 'Previous track',
        '1': 'Library',
        '2': 'Queue',
        '3': 'Player',
      });
    };

    const keyHandlers: Record<string, KeyHandler> = {
      ' ': handlePlayPause,
      'k': handlePlayPause,
      'arrowright': handleSeekForward,
      'l': handleSeekForward,
      'arrowleft': handleSeekBackward,
      'j': handleSeekBackward,
      'arrowup': handleVolumeUp,
      'arrowdown': handleVolumeDown,
      'n': handleNextTrack,
      'p': handlePreviousTrack,
      '1': handleNavigateTo('/library'),
      '2': handleNavigateTo('/queue'),
      '3': handleNavigateTo('/player'),
      '?': handleShowHelp,
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
        return;
      }

      const handler = keyHandlers[event.key.toLowerCase()];
      if (handler) {
        handler(event);
      }
    };

    globalThis.addEventListener('keydown', handleKeyDown);
    return () => globalThis.removeEventListener('keydown', handleKeyDown);
  }, [currentTrack, queue, isPlaying, togglePlayPause, seek, navigate, location]);
}
