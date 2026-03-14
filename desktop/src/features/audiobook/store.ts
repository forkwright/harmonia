import { create } from "zustand";
import { persist } from "zustand/middleware";

type FilterOption = "all" | "in_progress" | "completed" | "not_started";
type SortOption = "title" | "author" | "recently_listened" | "progress" | "date_added";

interface AudiobookLibraryState {
  filter: FilterOption;
  sort: SortOption;
  setFilter: (filter: FilterOption) => void;
  setSort: (sort: SortOption) => void;
}

export const useAudiobookLibraryStore = create<AudiobookLibraryState>()(
  persist(
    (set) => ({
      filter: "all",
      sort: "title",
      setFilter: (filter) => set({ filter }),
      setSort: (sort) => set({ sort }),
    }),
    { name: "harmonia-audiobook-library" }
  )
);

interface SpeedEntry {
  speed: number;
}

interface AudiobookPlayerState {
  // Per-audiobook speed keyed by audiobook ID
  speeds: Record<string, SpeedEntry>;
  // Most recently listened audiobook IDs (front = most recent)
  recentlyListened: string[];
  setSpeed: (audiobookId: string, speed: number) => void;
  speed: (audiobookId: string) => number;
  recordListened: (audiobookId: string) => void;
}

const DEFAULT_SPEED = 1.0;
const MAX_RECENT = 20;

export const useAudiobookPlayerStore = create<AudiobookPlayerState>()(
  persist(
    (set, get) => ({
      speeds: {},
      recentlyListened: [],
      setSpeed: (audiobookId, speed) =>
        set((s) => ({ speeds: { ...s.speeds, [audiobookId]: { speed } } })),
      speed: (audiobookId) => get().speeds[audiobookId]?.speed ?? DEFAULT_SPEED,
      recordListened: (audiobookId) =>
        set((s) => {
          const without = s.recentlyListened.filter((id) => id !== audiobookId);
          return { recentlyListened: [audiobookId, ...without].slice(0, MAX_RECENT) };
        }),
    }),
    { name: "harmonia-audiobook-player" }
  )
);

export type { FilterOption, SortOption };
