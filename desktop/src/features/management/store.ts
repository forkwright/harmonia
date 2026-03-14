import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { MediaType } from "../../types/management";

interface FilterState {
  status: string;
  qualityTier: string;
  hasMetadata: "all" | "yes" | "no";
}

interface ManagementState {
  selectedMediaType: MediaType;
  filters: FilterState;
  isAdmin: boolean;
  setSelectedMediaType: (mediaType: MediaType) => void;
  setFilter: (filter: Partial<FilterState>) => void;
  setIsAdmin: (isAdmin: boolean) => void;
  resetFilters: () => void;
}

const defaultFilters: FilterState = {
  status: "all",
  qualityTier: "all",
  hasMetadata: "all",
};

export const useManagementStore = create<ManagementState>()(
  persist(
    (set) => ({
      selectedMediaType: "music",
      filters: defaultFilters,
      isAdmin: false,
      setSelectedMediaType: (selectedMediaType) => set({ selectedMediaType }),
      setFilter: (filter) =>
        set((s) => ({ filters: { ...s.filters, ...filter } })),
      setIsAdmin: (isAdmin) => set({ isAdmin }),
      resetFilters: () => set({ filters: defaultFilters }),
    }),
    { name: "harmonia-management" },
  ),
);
