import { describe, it, expect, beforeEach, vi } from 'vitest'

vi.mock('../services/syncService', () => ({
  syncService: {
    reportProgress: vi.fn().mockResolvedValue(undefined),
  },
}))

vi.mock('../services/sessionManager', () => ({
  sessionManager: {
    startSession: vi.fn().mockResolvedValue('mock-session-id'),
    endSession: vi.fn().mockResolvedValue(undefined),
    updateSession: vi.fn().mockResolvedValue(undefined),
  },
}))

import { useAudiobookStore } from './audiobookStore'
import type { Audiobook, Chapter } from '../types'

const mockBook: Audiobook = {
  id: 1,
  title: 'Test Book',
  year: 2020,
  monitored: true,
  qualityProfileId: 1,
  added: '2026-01-01T00:00:00Z',
  authorId: 1,
  metadata: {
    description: 'A test audiobook',
    narrator: 'Test Narrator',
    narrators: ['Test Narrator'],
    durationMinutes: 600,
    isAbridged: false,
    genres: ['Fantasy'],
  },
}

const mockChapters: Chapter[] = [
  { title: 'Chapter 1', startTimeMs: 0, endTimeMs: 600000, index: 0 },
  { title: 'Chapter 2', startTimeMs: 600000, endTimeMs: 1200000, index: 1 },
  { title: 'Chapter 3', startTimeMs: 1200000, endTimeMs: 1800000, index: 2 },
]

describe('audiobookStore', () => {
  beforeEach(() => {
    useAudiobookStore.setState({
      authors: [],
      audiobooks: [],
      selectedAuthor: null,
      selectedAudiobook: null,
      chapters: [],
      currentAudiobook: null,
      currentChapter: null,
      positionMs: 0,
      isPlaying: false,
      loading: false,
      error: null,
      sleepTimerTarget: null,
      sleepTimerMode: null,
      bookSpeedMap: {},
      bookmarks: [],
    })
    localStorage.clear()
  })

  it('plays an audiobook and sets initial state', () => {
    const { playAudiobook } = useAudiobookStore.getState()
    playAudiobook(mockBook, 5000)

    const state = useAudiobookStore.getState()
    expect(state.currentAudiobook).toEqual(mockBook)
    expect(state.positionMs).toBe(5000)
    expect(state.isPlaying).toBe(true)
  })

  it('defaults position to 0 when playing', () => {
    const { playAudiobook } = useAudiobookStore.getState()
    playAudiobook(mockBook)

    expect(useAudiobookStore.getState().positionMs).toBe(0)
  })

  it('sets chapter and updates position', () => {
    const { setChapter } = useAudiobookStore.getState()
    setChapter(mockChapters[1]!)

    const state = useAudiobookStore.getState()
    expect(state.currentChapter).toEqual(mockChapters[1])
    expect(state.positionMs).toBe(600000)
  })

  it('auto-detects chapter from position', () => {
    useAudiobookStore.setState({ chapters: mockChapters })
    const { setPosition } = useAudiobookStore.getState()

    setPosition(800000) // 800s — within chapter 2
    expect(useAudiobookStore.getState().currentChapter?.index).toBe(1)

    setPosition(100000) // 100s — within chapter 1
    expect(useAudiobookStore.getState().currentChapter?.index).toBe(0)

    setPosition(1500000) // 1500s — within chapter 3
    expect(useAudiobookStore.getState().currentChapter?.index).toBe(2)
  })

  it('sets playing state', () => {
    const { setIsPlaying } = useAudiobookStore.getState()
    setIsPlaying(true)
    expect(useAudiobookStore.getState().isPlaying).toBe(true)
    setIsPlaying(false)
    expect(useAudiobookStore.getState().isPlaying).toBe(false)
  })

  it('selects and deselects author', () => {
    const author = { id: 1, name: 'Test', monitored: true, qualityProfileId: 1, added: '' }
    const { selectAuthor } = useAudiobookStore.getState()

    selectAuthor(author)
    expect(useAudiobookStore.getState().selectedAuthor).toEqual(author)

    selectAuthor(null)
    expect(useAudiobookStore.getState().selectedAuthor).toBeNull()
  })

  // Sleep timer tests
  describe('sleep timer', () => {
    it('sets a minutes-based sleep timer', () => {
      const now = Date.now()
      vi.spyOn(Date, 'now').mockReturnValue(now)

      const { setSleepTimer } = useAudiobookStore.getState()
      setSleepTimer(30)

      const state = useAudiobookStore.getState()
      expect(state.sleepTimerMode).toBe('minutes')
      expect(state.sleepTimerTarget).toBe(now + 30 * 60000)

      vi.restoreAllMocks()
    })

    it('sets end-of-chapter sleep timer', () => {
      const { setSleepTimer } = useAudiobookStore.getState()
      setSleepTimer('end-of-chapter')

      const state = useAudiobookStore.getState()
      expect(state.sleepTimerMode).toBe('end-of-chapter')
      expect(state.sleepTimerTarget).toBeNull()
    })

    it('clears sleep timer', () => {
      const { setSleepTimer, clearSleepTimer } = useAudiobookStore.getState()
      setSleepTimer(15)
      clearSleepTimer()

      const state = useAudiobookStore.getState()
      expect(state.sleepTimerMode).toBeNull()
      expect(state.sleepTimerTarget).toBeNull()
    })
  })

  // Per-book speed tests
  describe('per-book speed', () => {
    it('returns default speed 1 for unknown book', () => {
      const { getBookSpeed } = useAudiobookStore.getState()
      expect(getBookSpeed(999)).toBe(1)
    })

    it('saves and retrieves speed per book', () => {
      const { setBookSpeed, getBookSpeed } = useAudiobookStore.getState()
      setBookSpeed(1, 1.5)
      setBookSpeed(2, 0.75)

      expect(getBookSpeed(1)).toBe(1.5)
      expect(getBookSpeed(2)).toBe(0.75)
    })

    it('persists speed to localStorage', () => {
      const { setBookSpeed } = useAudiobookStore.getState()
      setBookSpeed(1, 2)

      const stored = JSON.parse(localStorage.getItem('akroasis_book_speeds') ?? '{}')
      expect(stored[1]).toBe(2)
    })
  })

  // Bookmark tests
  describe('bookmarks', () => {
    it('adds a bookmark at current position', () => {
      useAudiobookStore.setState({
        currentAudiobook: mockBook,
        positionMs: 300000,
        currentChapter: mockChapters[0]!,
        chapters: mockChapters,
      })

      const { addBookmark } = useAudiobookStore.getState()
      addBookmark('Great part')

      const bms = useAudiobookStore.getState().bookmarks
      expect(bms).toHaveLength(1)
      expect(bms[0]!.audiobookId).toBe(1)
      expect(bms[0]!.positionMs).toBe(300000)
      expect(bms[0]!.chapterTitle).toBe('Chapter 1')
      expect(bms[0]!.note).toBe('Great part')
    })

    it('adds bookmark with empty note by default', () => {
      useAudiobookStore.setState({
        currentAudiobook: mockBook,
        positionMs: 700000,
        currentChapter: mockChapters[1]!,
      })

      const { addBookmark } = useAudiobookStore.getState()
      addBookmark()

      expect(useAudiobookStore.getState().bookmarks[0]!.note).toBe('')
    })

    it('does nothing when no audiobook is playing', () => {
      const { addBookmark } = useAudiobookStore.getState()
      addBookmark()
      expect(useAudiobookStore.getState().bookmarks).toHaveLength(0)
    })

    it('removes a bookmark by id', () => {
      useAudiobookStore.setState({
        currentAudiobook: mockBook,
        positionMs: 100000,
        currentChapter: mockChapters[0]!,
      })

      const { addBookmark } = useAudiobookStore.getState()
      addBookmark('first')
      addBookmark('second')

      const bms = useAudiobookStore.getState().bookmarks
      expect(bms).toHaveLength(2)

      const { removeBookmark } = useAudiobookStore.getState()
      removeBookmark(bms[0]!.id)

      const remaining = useAudiobookStore.getState().bookmarks
      expect(remaining).toHaveLength(1)
      expect(remaining[0]!.note).toBe('second')
    })

    it('filters bookmarks by book id', () => {
      const book2 = { ...mockBook, id: 2, title: 'Other Book' }

      useAudiobookStore.setState({
        currentAudiobook: mockBook,
        positionMs: 100000,
        currentChapter: mockChapters[0]!,
      })
      useAudiobookStore.getState().addBookmark('book1')

      useAudiobookStore.setState({ currentAudiobook: book2 })
      useAudiobookStore.getState().addBookmark('book2')

      const { getBookmarksForBook } = useAudiobookStore.getState()
      expect(getBookmarksForBook(1)).toHaveLength(1)
      expect(getBookmarksForBook(2)).toHaveLength(1)
      expect(getBookmarksForBook(1)[0]!.note).toBe('book1')
    })

    it('persists bookmarks to localStorage', () => {
      useAudiobookStore.setState({
        currentAudiobook: mockBook,
        positionMs: 500000,
        currentChapter: mockChapters[0]!,
      })

      const { addBookmark } = useAudiobookStore.getState()
      addBookmark('test')

      const stored = JSON.parse(localStorage.getItem('akroasis_bookmarks') ?? '[]')
      expect(stored).toHaveLength(1)
      expect(stored[0].note).toBe('test')
    })
  })
})
