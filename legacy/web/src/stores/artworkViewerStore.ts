// Artwork lightbox viewer state
import { create } from 'zustand'

interface ArtworkViewerState {
  isOpen: boolean
  url: string
  open: (url: string) => void
  close: () => void
}

export const useArtworkViewer = create<ArtworkViewerState>((set) => ({
  isOpen: false,
  url: '',
  open: (url) => set({ isOpen: true, url }),
  close: () => set({ isOpen: false }),
}))
