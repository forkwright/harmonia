// Audiobook library and playback state
import { create } from 'zustand'
import type { Author, Audiobook, Bookmark, Chapter, ContinueItem } from '../types'
import { apiClient } from '../api/client'

function loadJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    return raw ? JSON.parse(raw) : fallback
  } catch {
    return fallback
  }
}

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

  // Sleep timer
  sleepTimerTarget: number | null
  sleepTimerMode: 'minutes' | 'end-of-chapter' | null

  // Per-book speed
  bookSpeedMap: Record<number, number>

  // Bookmarks
  bookmarks: Bookmark[]

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

  // Actions — sleep timer
  setSleepTimer: (value: number | 'end-of-chapter') => void
  clearSleepTimer: () => void

  // Actions — per-book speed
  getBookSpeed: (bookId: number) => number
  setBookSpeed: (bookId: number, speed: number) => void

  // Actions — bookmarks
  addBookmark: (note?: string) => void
  removeBookmark: (id: string) => void
  getBookmarksForBook: (bookId: number) => Bookmark[]
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

  sleepTimerTarget: null,
  sleepTimerMode: null,

  bookSpeedMap: loadJson<Record<number, number>>('akroasis_book_speeds', {}),
  bookmarks: loadJson<Bookmark[]>('akroasis_bookmarks', []),

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

  // Sleep timer
  setSleepTimer: (value) => {
    if (value === 'end-of-chapter') {
      set({ sleepTimerTarget: null, sleepTimerMode: 'end-of-chapter' })
    } else {
      set({ sleepTimerTarget: Date.now() + value * 60000, sleepTimerMode: 'minutes' })
    }
  },

  clearSleepTimer: () => {
    set({ sleepTimerTarget: null, sleepTimerMode: null })
  },

  // Per-book speed
  getBookSpeed: (bookId) => {
    return get().bookSpeedMap[bookId] ?? 1
  },

  setBookSpeed: (bookId, speed) => {
    const map = { ...get().bookSpeedMap, [bookId]: speed }
    localStorage.setItem('akroasis_book_speeds', JSON.stringify(map))
    set({ bookSpeedMap: map })
  },

  // Bookmarks
  addBookmark: (note = '') => {
    const { currentAudiobook, positionMs, currentChapter, bookmarks } = get()
    if (!currentAudiobook) return
    const bookmark: Bookmark = {
      id: crypto.randomUUID(),
      audiobookId: currentAudiobook.id,
      positionMs,
      chapterTitle: currentChapter?.title ?? '',
      note,
      createdAt: new Date().toISOString(),
    }
    const updated = [...bookmarks, bookmark]
    localStorage.setItem('akroasis_bookmarks', JSON.stringify(updated))
    set({ bookmarks: updated })
  },

  removeBookmark: (id) => {
    const updated = get().bookmarks.filter((b) => b.id !== id)
    localStorage.setItem('akroasis_bookmarks', JSON.stringify(updated))
    set({ bookmarks: updated })
  },

  getBookmarksForBook: (bookId) => {
    return get().bookmarks.filter((b) => b.audiobookId === bookId)
  },
}))
