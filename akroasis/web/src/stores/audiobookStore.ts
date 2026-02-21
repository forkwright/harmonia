// Audiobook library and playback state
import { create } from 'zustand'
import type { Author, Audiobook, Chapter, ContinueItem } from '../types'
import { apiClient } from '../api/client'

interface AudiobookState {
  // Library
  authors: Author[]
  audiobooks: Audiobook[]
  selectedAuthor: Author | null
  selectedAudiobook: Audiobook | null
  chapters: Chapter[]
  continueItems: ContinueItem[]

  // Playback
  currentAudiobook: Audiobook | null
  currentChapter: Chapter | null
  positionMs: number
  isPlaying: boolean

  // Loading
  loading: boolean
  error: string | null

  // Actions — library
  loadAuthors: () => Promise<void>
  loadAudiobooks: () => Promise<void>
  loadAudiobooksByAuthor: (authorId: number) => Promise<void>
  loadChapters: (mediaFileId: number) => Promise<void>
  loadContinueListening: () => Promise<void>
  selectAuthor: (author: Author | null) => void
  selectAudiobook: (audiobook: Audiobook | null) => void

  // Actions — playback
  playAudiobook: (audiobook: Audiobook, positionMs?: number) => void
  setChapter: (chapter: Chapter) => void
  setPosition: (positionMs: number) => void
  setIsPlaying: (playing: boolean) => void
  saveProgress: () => Promise<void>
}

export const useAudiobookStore = create<AudiobookState>((set, get) => ({
  authors: [],
  audiobooks: [],
  selectedAuthor: null,
  selectedAudiobook: null,
  chapters: [],
  continueItems: [],

  currentAudiobook: null,
  currentChapter: null,
  positionMs: 0,
  isPlaying: false,

  loading: false,
  error: null,

  loadAuthors: async () => {
    try {
      set({ loading: true, error: null })
      const result = await apiClient.getAuthors()
      set({ authors: result.items, loading: false })
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to load authors', loading: false })
    }
  },

  loadAudiobooks: async () => {
    try {
      set({ loading: true, error: null })
      const result = await apiClient.getAudiobooks()
      set({ audiobooks: result.items, loading: false })
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to load audiobooks', loading: false })
    }
  },

  loadAudiobooksByAuthor: async (authorId: number) => {
    try {
      set({ loading: true, error: null })
      const audiobooks = await apiClient.getAudiobooksByAuthor(authorId)
      set({ audiobooks, loading: false })
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to load audiobooks', loading: false })
    }
  },

  loadChapters: async (mediaFileId: number) => {
    try {
      const chapters = await apiClient.getChapters(mediaFileId)
      set({ chapters })
    } catch {
      set({ chapters: [] })
    }
  },

  loadContinueListening: async () => {
    try {
      const items = await apiClient.getContinueListening()
      set({ continueItems: items })
    } catch {
      set({ continueItems: [] })
    }
  },

  selectAuthor: (author) => set({ selectedAuthor: author }),
  selectAudiobook: (audiobook) => set({ selectedAudiobook: audiobook }),

  playAudiobook: (audiobook, positionMs = 0) => {
    set({
      currentAudiobook: audiobook,
      positionMs,
      isPlaying: true,
      currentChapter: null,
    })
  },

  setChapter: (chapter) => set({ currentChapter: chapter, positionMs: chapter.startTimeMs }),
  setPosition: (positionMs) => {
    const { chapters } = get()
    // Auto-detect current chapter
    const current = [...chapters].reverse().find((ch: Chapter) => positionMs >= ch.startTimeMs) ?? null
    set({ positionMs, currentChapter: current })
  },
  setIsPlaying: (playing) => set({ isPlaying: playing }),

  saveProgress: async () => {
    const { currentAudiobook, positionMs } = get()
    if (!currentAudiobook) return
    const totalMs = (currentAudiobook.metadata.durationMinutes ?? 0) * 60 * 1000
    try {
      await apiClient.updateProgress(currentAudiobook.id, positionMs, totalMs)
    } catch {
      // Silent fail — progress save is best-effort
    }
  },
}))
