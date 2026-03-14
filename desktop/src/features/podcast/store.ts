/** Global podcast playback state: speed preference and currently playing episode. */

import { create } from "zustand";
import { persist } from "zustand/middleware";

interface PodcastState {
  speed: number;
  currentEpisodeId: string | null;
  isPlaying: boolean;
  positionMs: number;
  setSpeed: (speed: number) => void;
  setCurrentEpisodeId: (id: string | null) => void;
  setIsPlaying: (playing: boolean) => void;
  setPositionMs: (ms: number) => void;
}

export const usePodcastStore = create<PodcastState>()(
  persist(
    (set) => ({
      speed: 1.0,
      currentEpisodeId: null,
      isPlaying: false,
      positionMs: 0,
      setSpeed: (speed) => set({ speed }),
      setCurrentEpisodeId: (id) => set({ currentEpisodeId: id }),
      setIsPlaying: (playing) => set({ isPlaying: playing }),
      setPositionMs: (ms) => set({ positionMs: ms }),
    }),
    {
      name: "harmonia-podcast",
      // WHY: position is transient — restore speed preference only.
      partialize: (s) => ({ speed: s.speed }),
    }
  )
);
