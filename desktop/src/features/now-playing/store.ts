import { create } from "zustand";
import { persist } from "zustand/middleware";

interface NowPlayingState {
  expanded: boolean;
  volume: number;
  setExpanded: (expanded: boolean) => void;
  setVolume: (volume: number) => void;
}

export const useNowPlayingStore = create<NowPlayingState>()(
  persist(
    (set) => ({
      expanded: false,
      volume: 1.0,
      setExpanded: (expanded) => set({ expanded }),
      setVolume: (volume) => set({ volume }),
    }),
    { name: "harmonia-now-playing" }
  )
);
