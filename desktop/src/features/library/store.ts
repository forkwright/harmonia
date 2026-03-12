import { create } from "zustand";
import { persist } from "zustand/middleware";

type SortOption = "title" | "year" | "added";

interface LibraryState {
  token: string;
  sort: SortOption;
  setToken: (token: string) => void;
  setSort: (sort: SortOption) => void;
}

export const useLibraryStore = create<LibraryState>()(
  persist(
    (set) => ({
      token: "",
      sort: "title",
      setToken: (token) => set({ token }),
      setSort: (sort) => set({ sort }),
    }),
    { name: "harmonia-library" }
  )
);

export type { SortOption };
