// Global keyboard shortcuts for media control
import { useEffect, useCallback } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer';
import { useNavigate } from 'react-router-dom';

type KeyHandler = (event: KeyboardEvent) => void;

function createPlayPauseHandler(togglePlayPause: () => void): KeyHandler {
  return (event) => {
    event.preventDefault();
    togglePlayPause();
  };
}

function createSeekForwardHandler(seek: (time: number) => void): KeyHandler {
  return (event) => {
    event.preventDefault();
    const currentPos = usePlayerStore.getState().position / 1000;
    seek(currentPos + 10);
  };
}

function createSeekBackwardHandler(seek: (time: number) => void): KeyHandler {
  return (event) => {
    event.preventDefault();
    const currentPos = usePlayerStore.getState().position / 1000;
    seek(Math.max(0, currentPos - 10));
  };
}

function createVolumeUpHandler(): KeyHandler {
  return (event) => {
    event.preventDefault();
    const currentVolume = usePlayerStore.getState().volume;
    usePlayerStore.getState().setVolume(Math.min(1, currentVolume + 0.1));
  };
}

function createVolumeDownHandler(): KeyHandler {
  return (event) => {
    event.preventDefault();
    const currentVolume = usePlayerStore.getState().volume;
    usePlayerStore.getState().setVolume(Math.max(0, currentVolume - 0.1));
  };
}

function createNextTrackHandler(): KeyHandler {
  return (event) => {
    event.preventDefault();
    const { currentTrack, queue } = usePlayerStore.getState();
    if (!currentTrack || queue.length === 0) return;

    const currentIndex = queue.findIndex((t) => t.id === currentTrack.id);
    if (currentIndex !== -1 && currentIndex < queue.length - 1) {
      usePlayerStore.getState().setCurrentTrack(queue[currentIndex + 1]);
    }
  };
}

function createPreviousTrackHandler(): KeyHandler {
  return (event) => {
    event.preventDefault();
    const { currentTrack, queue } = usePlayerStore.getState();
    if (!currentTrack || queue.length === 0) return;

    const currentIndex = queue.findIndex((t) => t.id === currentTrack.id);
    if (currentIndex > 0) {
      usePlayerStore.getState().setCurrentTrack(queue[currentIndex - 1]);
    }
  };
}

function createNavigateHandler(navigate: (path: string) => void, path: string): KeyHandler {
  return (event) => {
    if (!event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      navigate(path);
    }
  };
}

function buildKeyHandlers(
  togglePlayPause: () => void,
  seek: (time: number) => void,
  navigate: (path: string) => void
): Record<string, KeyHandler> {
  const playPause = createPlayPauseHandler(togglePlayPause);
  const seekForward = createSeekForwardHandler(seek);
  const seekBackward = createSeekBackwardHandler(seek);

  return {
    ' ': playPause,
    'k': playPause,
    'arrowright': seekForward,
    'l': seekForward,
    'arrowleft': seekBackward,
    'j': seekBackward,
    'arrowup': createVolumeUpHandler(),
    'arrowdown': createVolumeDownHandler(),
    'n': createNextTrackHandler(),
    'p': createPreviousTrackHandler(),
    '1': createNavigateHandler(navigate, '/library'),
    '2': createNavigateHandler(navigate, '/queue'),
    '3': createNavigateHandler(navigate, '/player'),
  };
}

function isInputElement(target: EventTarget | null): boolean {
  return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
}

export function useKeyboardShortcuts() {
  const { togglePlayPause, seek } = useWebAudioPlayer();
  const navigate = useNavigate();

  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (isInputElement(event.target)) return;

      const keyHandlers = buildKeyHandlers(togglePlayPause, seek, navigate);
      const handler = keyHandlers[event.key.toLowerCase()];
      if (handler) {
        handler(event);
      }
    },
    [togglePlayPause, seek, navigate]
  );

  useEffect(() => {
    globalThis.addEventListener('keydown', handleKeyDown);
    return () => globalThis.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}
