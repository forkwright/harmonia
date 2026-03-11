// Cross-media search state
import { create } from 'zustand'
import { apiClient } from '../api/client'
import { useAudiobookStore } from './audiobookStore'
import { usePodcastStore } from './podcastStore'
import type { UnifiedSearchResult } from '../types'

interface SearchState {
  query: string
  results: UnifiedSearchResult[]
  isSearching: boolean
  isOpen: boolean
  selectedIndex: number

  setQuery: (query: string) => void
  search: (query: string) => Promise<void>
  setOpen: (open: boolean) => void
  setSelectedIndex: (index: number) => void
  clear: () => void
}

export const useSearchStore = create<SearchState>((set) => ({
  query: '',
  results: [],
  isSearching: false,
  isOpen: false,
  selectedIndex: -1,

  setQuery: (query: string) => {
    set({ query })
  },

  search: async (query: string) => {
    if (!query.trim()) {
      set({ results: [], isSearching: false, isOpen: false })
      return
    }

    set({ isSearching: true })

    try {
      const lower = query.toLowerCase()
      const results: UnifiedSearchResult[] = []

      const trackResults = await apiClient.search(query, 10)
      for (const r of trackResults) {
        results.push({
          id: r.trackId,
          type: 'track',
          title: r.title,
          subtitle: [r.artist, r.album].filter(Boolean).join(' — '),
        })
      }

      const { audiobooks, authors } = useAudiobookStore.getState()
      const authorMap = new Map(authors.map((a) => [a.id, a.name]))
      for (const ab of audiobooks) {
        const authorName = ab.authorId != null ? authorMap.get(ab.authorId) : undefined
        const searchable = `${ab.title} ${authorName ?? ''} ${ab.metadata.narrator ?? ''}`.toLowerCase()
        if (searchable.includes(lower)) {
          results.push({
            id: ab.id,
            type: 'audiobook',
            title: ab.title,
            subtitle: authorName ?? ab.metadata.narrator ?? '',
          })
        }
      }

      const { shows } = usePodcastStore.getState()
      for (const show of shows) {
        const searchable = `${show.title} ${show.author ?? ''}`.toLowerCase()
        if (searchable.includes(lower)) {
          results.push({
            id: show.id,
            type: 'podcast',
            title: show.title,
            subtitle: show.author ?? '',
            coverUrl: show.imageUrl,
          })
        }
      }

      set({ results: results.slice(0, 20), isSearching: false, isOpen: true, selectedIndex: -1 })
    } catch {
      set({ results: [], isSearching: false, isOpen: false })
    }
  },

  setOpen: (isOpen: boolean) => {
    set({ isOpen, selectedIndex: -1 })
  },

  setSelectedIndex: (selectedIndex: number) => {
    set({ selectedIndex })
  },

  clear: () => {
    set({ query: '', results: [], isSearching: false, isOpen: false, selectedIndex: -1 })
  },
}))
