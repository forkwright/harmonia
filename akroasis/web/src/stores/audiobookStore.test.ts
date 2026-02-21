import { describe, it, expect, beforeEach } from 'vitest'
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
      continueItems: [],
      currentAudiobook: null,
      currentChapter: null,
      positionMs: 0,
      isPlaying: false,
      loading: false,
      error: null,
    })
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
})
