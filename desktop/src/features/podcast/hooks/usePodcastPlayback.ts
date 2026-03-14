/** Tauri IPC commands for podcast playback plus keyboard shortcuts. */

import { useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { usePodcastStore } from "../store";
import { useLibraryStore } from "../../library/store";
import { api } from "../../../api/client";

const SKIP_FORWARD_SECONDS = 30;
const SKIP_BACKWARD_SECONDS = 15;
const SPEED_STEP = 0.1;
const MIN_SPEED = 0.5;
const MAX_SPEED = 3.0;

export function usePodcastPlayback() {
  const token = useLibraryStore((s) => s.token);
  const { speed, currentEpisodeId, isPlaying, setSpeed, setCurrentEpisodeId, setIsPlaying } =
    usePodcastStore();

  const playEpisode = useCallback(
    async (episodeId: string) => {
      await invoke("podcast_play_episode", { episodeId });
      setCurrentEpisodeId(episodeId);
      setIsPlaying(true);
    },
    [setCurrentEpisodeId, setIsPlaying],
  );

  const resumeEpisode = useCallback(
    async (episodeId: string, positionMs: number) => {
      await invoke("podcast_resume_episode", { episodeId, positionMs });
      setCurrentEpisodeId(episodeId);
      setIsPlaying(true);
    },
    [setCurrentEpisodeId, setIsPlaying],
  );

  const pause = useCallback(async () => {
    if (!currentEpisodeId) return;
    // WHY: P3-11 audio engine integration point — pause is a no-op until then.
    setIsPlaying(false);
    if (currentEpisodeId && token) {
      const pos = usePodcastStore.getState().positionMs;
      void api.updateEpisodeProgress(currentEpisodeId, { positionMs: pos }, token);
    }
  }, [currentEpisodeId, setIsPlaying, token]);

  const skipForward = useCallback(async () => {
    await invoke("podcast_skip_forward", { seconds: SKIP_FORWARD_SECONDS });
  }, []);

  const skipBackward = useCallback(async () => {
    await invoke("podcast_skip_backward", { seconds: SKIP_BACKWARD_SECONDS });
  }, []);

  const changeSpeed = useCallback(
    async (newSpeed: number) => {
      const clamped = Math.min(MAX_SPEED, Math.max(MIN_SPEED, newSpeed));
      await invoke("podcast_set_speed", { speed: clamped });
      setSpeed(clamped);
    },
    [setSpeed],
  );

  // Keyboard shortcuts active whenever an episode is loaded.
  useEffect(() => {
    if (!currentEpisodeId) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      const active = document.activeElement;
      if (
        active instanceof HTMLInputElement ||
        active instanceof HTMLTextAreaElement ||
        active instanceof HTMLSelectElement
      ) {
        return;
      }

      switch (e.key) {
        case " ":
          e.preventDefault();
          if (isPlaying) {
            void pause();
          } else if (currentEpisodeId) {
            void resumeEpisode(currentEpisodeId, usePodcastStore.getState().positionMs);
          }
          break;
        case "ArrowRight":
          e.preventDefault();
          void skipForward();
          break;
        case "ArrowLeft":
          e.preventDefault();
          void skipBackward();
          break;
        case "]":
          void changeSpeed(speed + SPEED_STEP);
          break;
        case "[":
          void changeSpeed(speed - SPEED_STEP);
          break;
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [currentEpisodeId, isPlaying, speed, pause, resumeEpisode, skipForward, skipBackward, changeSpeed]);

  return {
    currentEpisodeId,
    isPlaying,
    speed,
    playEpisode,
    resumeEpisode,
    pause,
    skipForward,
    skipBackward,
    changeSpeed,
  };
}
