// Global keyboard shortcuts for media control
import { useEffect } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer';
import { useNavigate, useLocation } from 'react-router-dom';

export function useKeyboardShortcuts() {
  const { currentTrack, queue, isPlaying } = usePlayerStore();
  const { togglePlayPause, seek } = useWebAudioPlayer();
  const navigate = useNavigate();
  const location = useLocation();

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Don't trigger if user is typing in an input
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
        return;
      }

      switch (event.key.toLowerCase()) {
        case ' ':
        case 'k':
          // Space or K: Play/Pause
          event.preventDefault();
          togglePlayPause();
          break;

        case 'arrowright':
        case 'l':
          // Right arrow or L: Seek forward 10s
          event.preventDefault();
          if (currentTrack) {
            const currentPos = usePlayerStore.getState().position / 1000;
            seek(currentPos + 10);
          }
          break;

        case 'arrowleft':
        case 'j':
          // Left arrow or J: Seek backward 10s
          event.preventDefault();
          if (currentTrack) {
            const currentPos = usePlayerStore.getState().position / 1000;
            seek(Math.max(0, currentPos - 10));
          }
          break;

        case 'arrowup':
          // Up arrow: Volume up
          event.preventDefault();
          {
            const currentVolume = usePlayerStore.getState().volume;
            usePlayerStore.getState().setVolume(Math.min(1, currentVolume + 0.1));
          }
          break;

        case 'arrowdown':
          // Down arrow: Volume down
          event.preventDefault();
          {
            const currentVolume = usePlayerStore.getState().volume;
            usePlayerStore.getState().setVolume(Math.max(0, currentVolume - 0.1));
          }
          break;

        case 'n':
          // N: Next track
          event.preventDefault();
          if (currentTrack && queue.length > 0) {
            const currentIndex = queue.findIndex((t) => t.id === currentTrack.id);
            if (currentIndex !== -1 && currentIndex < queue.length - 1) {
              const nextTrack = queue[currentIndex + 1];
              usePlayerStore.getState().setCurrentTrack(nextTrack);
            }
          }
          break;

        case 'p':
          // P: Previous track
          event.preventDefault();
          if (currentTrack && queue.length > 0) {
            const currentIndex = queue.findIndex((t) => t.id === currentTrack.id);
            if (currentIndex > 0) {
              const prevTrack = queue[currentIndex - 1];
              usePlayerStore.getState().setCurrentTrack(prevTrack);
            }
          }
          break;

        case '1':
          // 1: Navigate to Library
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            navigate('/library');
          }
          break;

        case '2':
          // 2: Navigate to Queue
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            navigate('/queue');
          }
          break;

        case '3':
          // 3: Navigate to Player
          if (!event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            navigate('/player');
          }
          break;

        case '?':
          // ?: Show keyboard shortcuts help (future)
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
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [currentTrack, queue, isPlaying, togglePlayPause, seek, navigate, location]);
}
