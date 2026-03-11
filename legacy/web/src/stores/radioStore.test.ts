// Radio store state management tests
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { useRadioStore } from './radioStore'
import { usePlayerStore } from './playerStore'
import type { Track } from '../types'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const makeTrack = (overrides: Partial<Track> = {}): Track => ({
  id: 1,
  title: 'Time',
  artist: 'Pink Floyd',
  album: 'The Dark Side of the Moon',
  duration: 413000,
  fileSize: 76422144,
  format: 'FLAC',
  bitrate: 1411,
  sampleRate: 96000,
  bitDepth: 24,
  channels: 2,
  ...overrides,
})

function resetStores() {
  useRadioStore.setState({
    radioMode: false,
    radioSeed: null,
    loading: false,
    error: null,
  })
  usePlayerStore.getState().clearQueue()
  usePlayerStore.getState().setCurrentTrack(null)
  usePlayerStore.getState().setIsPlaying(false)
}

// ---------------------------------------------------------------------------
// Initial state
// ---------------------------------------------------------------------------

describe('radioStore initial state', () => {
  beforeEach(resetStores)

  it('starts with radio off', () => {
    const state = useRadioStore.getState()
    expect(state.radioMode).toBe(false)
    expect(state.radioSeed).toBeNull()
    expect(state.loading).toBe(false)
    expect(state.error).toBeNull()
  })
})

// ---------------------------------------------------------------------------
// stopRadio
// ---------------------------------------------------------------------------

describe('stopRadio', () => {
  beforeEach(resetStores)

  it('resets all radio state', () => {
    useRadioStore.setState({
      radioMode: true,
      radioSeed: makeTrack(),
      loading: true,
      error: 'oops',
    })

    useRadioStore.getState().stopRadio()

    const state = useRadioStore.getState()
    expect(state.radioMode).toBe(false)
    expect(state.radioSeed).toBeNull()
    expect(state.loading).toBe(false)
    expect(state.error).toBeNull()
  })
})

// ---------------------------------------------------------------------------
// startRadio — API key not configured
// ---------------------------------------------------------------------------

describe('startRadio when Last.fm not configured', () => {
  beforeEach(resetStores)

  it('sets error and leaves radioMode false when API key absent', async () => {
    // VITE_LASTFM_API_KEY is not set in test environment
    const seed = makeTrack()
    await useRadioStore.getState().startRadio(seed)

    const state = useRadioStore.getState()
    expect(state.radioMode).toBe(false)
    expect(state.error).toContain('API key')
  })
})

// ---------------------------------------------------------------------------
// startRadio — API key configured, buildRadioTracks mocked via apiClient spies
// ---------------------------------------------------------------------------

describe('startRadio with Last.fm configured', () => {
  beforeEach(() => {
    resetStores()
    vi.stubEnv('VITE_LASTFM_API_KEY', 'test-key-123')
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.unstubAllEnvs()
    vi.unstubAllGlobals()
    resetStores()
  })

  it('sets radioMode and queue when tracks are found', async () => {
    const { apiClient } = await import('../api/client')
    const seed = makeTrack({ id: 1 })
    const radioTrack2 = makeTrack({ id: 2, title: 'Money', artist: 'Pink Floyd' })
    const radioTrack3 = makeTrack({ id: 3, title: 'Brain Damage', artist: 'Pink Floyd' })

    // Mock Last.fm HTTP response
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        similartracks: {
          track: [
            { name: 'Money', artist: { name: 'Pink Floyd' } },
            { name: 'Brain Damage', artist: { name: 'Pink Floyd' } },
          ],
          '@attr': { artist: 'Pink Floyd' },
        },
      }),
    }))

    vi.spyOn(apiClient, 'search')
      .mockResolvedValueOnce([{ trackId: 2, title: 'Money', artist: 'Pink Floyd', album: 'DSOTM', trackNumber: 1, discNumber: 1, lossless: true, relevanceScore: 1 }])
      .mockResolvedValueOnce([{ trackId: 3, title: 'Brain Damage', artist: 'Pink Floyd', album: 'DSOTM', trackNumber: 1, discNumber: 1, lossless: true, relevanceScore: 1 }])

    vi.spyOn(apiClient, 'getTrack')
      .mockResolvedValueOnce(radioTrack2)
      .mockResolvedValueOnce(radioTrack3)

    await useRadioStore.getState().startRadio(seed)

    const state = useRadioStore.getState()
    expect(state.radioMode).toBe(true)
    expect(state.radioSeed).toEqual(seed)
    expect(state.loading).toBe(false)
    expect(state.error).toBeNull()

    const queue = usePlayerStore.getState().queue
    expect(queue.length).toBeGreaterThan(0)
  })

  it('sets error and disables radio when no library matches found', async () => {
    const { apiClient } = await import('../api/client')
    const seed = makeTrack({ id: 1 })

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        similartracks: {
          track: [{ name: 'Unknown Track', artist: { name: 'Ghost Artist' } }],
          '@attr': { artist: 'Ghost' },
        },
      }),
    }))

    vi.spyOn(apiClient, 'search').mockResolvedValue([]) // no matches

    await useRadioStore.getState().startRadio(seed)

    const state = useRadioStore.getState()
    expect(state.radioMode).toBe(false)
    expect(state.error).toBeTruthy()
  })

  it('sets error when Last.fm fetch fails', async () => {
    const seed = makeTrack({ id: 1 })

    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network failure')))

    await useRadioStore.getState().startRadio(seed)

    const state = useRadioStore.getState()
    expect(state.radioMode).toBe(false)
    expect(state.error).toBeTruthy()
  })
})

// ---------------------------------------------------------------------------
// replenishIfNeeded — guard conditions (no Last.fm needed)
// ---------------------------------------------------------------------------

describe('replenishIfNeeded — guard conditions', () => {
  beforeEach(resetStores)
  afterEach(() => vi.restoreAllMocks())

  it('does nothing when radio is not active', async () => {
    useRadioStore.setState({ radioMode: false })
    const seed = makeTrack()
    await useRadioStore.getState().replenishIfNeeded(seed)
    expect(useRadioStore.getState().loading).toBe(false)
  })

  it('does nothing when radioSeed is null', async () => {
    useRadioStore.setState({ radioMode: true, radioSeed: null })
    await useRadioStore.getState().replenishIfNeeded(null)
    expect(useRadioStore.getState().loading).toBe(false)
  })

  it('does nothing when already loading', async () => {
    const seed = makeTrack({ id: 1 })
    useRadioStore.setState({ radioMode: true, radioSeed: seed, loading: true })
    usePlayerStore.getState().setQueue([makeTrack({ id: 10 })])

    await useRadioStore.getState().replenishIfNeeded(seed)
    expect(useRadioStore.getState().loading).toBe(true)
  })

  it('does nothing when queue is above threshold', async () => {
    const seed = makeTrack({ id: 1 })
    useRadioStore.setState({ radioMode: true, radioSeed: seed })

    const tracks = Array.from({ length: 5 }, (_, i) => makeTrack({ id: i + 10 }))
    usePlayerStore.getState().setQueue(tracks)

    await useRadioStore.getState().replenishIfNeeded(seed)
    expect(useRadioStore.getState().loading).toBe(false)
  })
})

// ---------------------------------------------------------------------------
// replenishIfNeeded — with Last.fm configured
// ---------------------------------------------------------------------------

describe('replenishIfNeeded — with Last.fm configured', () => {
  beforeEach(() => {
    resetStores()
    vi.stubEnv('VITE_LASTFM_API_KEY', 'test-key-123')
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.unstubAllEnvs()
    vi.unstubAllGlobals()
    resetStores()
  })

  it('appends new tracks to queue when below threshold', async () => {
    const { apiClient } = await import('../api/client')
    const seed = makeTrack({ id: 1 })
    const newTrack = makeTrack({ id: 50, title: 'New Radio Track', artist: 'Someone' })

    useRadioStore.setState({ radioMode: true, radioSeed: seed, loading: false, error: null })
    usePlayerStore.getState().setQueue([makeTrack({ id: 10 })]) // 1 track — below threshold of 2

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        similartracks: {
          track: [{ name: 'New Radio Track', artist: { name: 'Someone' } }],
          '@attr': { artist: 'Pink Floyd' },
        },
      }),
    }))

    vi.spyOn(apiClient, 'search').mockResolvedValue([
      { trackId: 50, title: 'New Radio Track', artist: 'Someone', album: 'X', trackNumber: 1, discNumber: 1, lossless: false, relevanceScore: 1 },
    ])
    vi.spyOn(apiClient, 'getTrack').mockResolvedValue(newTrack)

    await useRadioStore.getState().replenishIfNeeded(seed)

    const queue = usePlayerStore.getState().queue
    expect(queue.some((t) => t.id === 50)).toBe(true)
  })

  it('does not add duplicates when replenishing', async () => {
    const { apiClient } = await import('../api/client')
    const seed = makeTrack({ id: 1 })
    const existingTrack = makeTrack({ id: 10, title: 'Already Queued', artist: 'Artist' })

    useRadioStore.setState({ radioMode: true, radioSeed: seed, loading: false, error: null })
    usePlayerStore.getState().setQueue([existingTrack]) // below threshold

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        similartracks: {
          track: [{ name: 'Already Queued', artist: { name: 'Artist' } }],
          '@attr': { artist: 'Seed' },
        },
      }),
    }))

    vi.spyOn(apiClient, 'search').mockResolvedValue([
      { trackId: 10, title: 'Already Queued', artist: 'Artist', album: 'X', trackNumber: 1, discNumber: 1, lossless: false, relevanceScore: 1 },
    ])
    vi.spyOn(apiClient, 'getTrack').mockResolvedValue(existingTrack)

    await useRadioStore.getState().replenishIfNeeded(seed)

    const queue = usePlayerStore.getState().queue
    const dupeCount = queue.filter((t) => t.id === 10).length
    expect(dupeCount).toBe(1)
  })
})
