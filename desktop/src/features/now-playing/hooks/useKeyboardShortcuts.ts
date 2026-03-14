import { useEffect } from "react";
import type { PlaybackState } from "../../../types/playback";
import { usePlayback } from "./usePlayback";

function isTypingTarget(target: EventTarget | null): boolean {
  if (!target) return false;
  const el = target as HTMLElement;
  return (
    el.tagName === "INPUT" ||
    el.tagName === "TEXTAREA" ||
    el.tagName === "SELECT" ||
    el.isContentEditable
  );
}

export function useKeyboardShortcuts(state: PlaybackState) {
  const { pause, resume, seek, nextTrack, previousTrack, setVolume } =
    usePlayback();
  const { setShuffle, setRepeatMode } = usePlayback();

  useEffect(() => {
    async function onKeyDown(e: KeyboardEvent) {
      if (isTypingTarget(e.target)) return;

      switch (e.key) {
        case " ":
          e.preventDefault();
          if (state.status === "playing") await pause();
          else if (state.status === "paused") await resume();
          break;
        case "ArrowRight":
          e.preventDefault();
          await seek(Math.min(state.position_ms + 10_000, state.duration_ms));
          break;
        case "ArrowLeft":
          e.preventDefault();
          await seek(Math.max(state.position_ms - 10_000, 0));
          break;
        case "ArrowUp":
          e.preventDefault();
          await setVolume(Math.min(state.volume + 0.05, 1.0));
          break;
        case "ArrowDown":
          e.preventDefault();
          await setVolume(Math.max(state.volume - 0.05, 0.0));
          break;
        case "m":
        case "M":
          await setVolume(state.volume > 0 ? 0 : 1.0);
          break;
        case "n":
        case "N":
          await nextTrack();
          break;
        case "p":
        case "P":
          await previousTrack();
          break;
        case "s":
        case "S":
          await setShuffle(!state.shuffle);
          break;
        case "r":
        case "R": {
          const modes = ["off", "all", "one"] as const;
          const idx = modes.indexOf(state.repeat_mode);
          await setRepeatMode(modes[(idx + 1) % modes.length]);
          break;
        }
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [state, pause, resume, seek, nextTrack, previousTrack, setVolume, setShuffle, setRepeatMode]);
}
