import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { fetchLrclib } from './lrclib'

describe('fetchLrclib', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('builds correct query string from track metadata', async () => {
    const mockFetch = vi.mocked(fetch)
    mockFetch.mockResolvedValueOnce(
      new Response(JSON.stringify({ id: 1, syncedLyrics: '[00:01.00]Line', plainLyrics: 'Line', instrumental: false, trackName: 'T', artistName: 'A', albumName: 'B', duration: 240 }), { status: 200 }),
    )

    await fetchLrclib('Artist Name', 'Track Title', 'Album Name', 240)

    const calledUrl = mockFetch.mock.calls[0][0] as string
    expect(calledUrl).toContain('artist_name=Artist+Name')
    expect(calledUrl).toContain('track_name=Track+Title')
    expect(calledUrl).toContain('album_name=Album+Name')
    expect(calledUrl).toContain('duration=240')
  })

  it('rounds duration to nearest integer', async () => {
    const mockFetch = vi.mocked(fetch)
    mockFetch.mockResolvedValueOnce(
      new Response(JSON.stringify({ id: 1, syncedLyrics: null, plainLyrics: null, instrumental: false, trackName: 'T', artistName: 'A', albumName: 'B', duration: 241 }), { status: 200 }),
    )

    await fetchLrclib('A', 'T', 'B', 241.7)

    const calledUrl = mockFetch.mock.calls[0][0] as string
    expect(calledUrl).toContain('duration=242')
  })

  it('returns null on 404', async () => {
    const mockFetch = vi.mocked(fetch)
    mockFetch.mockResolvedValueOnce(new Response(null, { status: 404 }))

    const result = await fetchLrclib('Unknown', 'Unknown', 'Unknown', 180)
    expect(result).toBeNull()
  })

  it('returns parsed response body on success', async () => {
    const payload = {
      id: 42,
      trackName: 'Song',
      artistName: 'Band',
      albumName: 'Record',
      duration: 200,
      instrumental: false,
      plainLyrics: 'Plain text lyrics',
      syncedLyrics: '[00:01.00]Plain text lyrics',
    }
    const mockFetch = vi.mocked(fetch)
    mockFetch.mockResolvedValueOnce(new Response(JSON.stringify(payload), { status: 200 }))

    const result = await fetchLrclib('Band', 'Song', 'Record', 200)
    expect(result).toEqual(payload)
  })

  it('throws on non-404 server errors', async () => {
    const mockFetch = vi.mocked(fetch)
    mockFetch.mockResolvedValueOnce(new Response(null, { status: 500, statusText: 'Internal Server Error' }))

    await expect(fetchLrclib('A', 'T', 'B', 180)).rejects.toThrow('LRCLIB error 500')
  })

  it('returns response with null lyrics fields', async () => {
    const payload = {
      id: 10,
      trackName: 'Instrumental',
      artistName: 'Composer',
      albumName: 'Score',
      duration: 300,
      instrumental: true,
      plainLyrics: null,
      syncedLyrics: null,
    }
    const mockFetch = vi.mocked(fetch)
    mockFetch.mockResolvedValueOnce(new Response(JSON.stringify(payload), { status: 200 }))

    const result = await fetchLrclib('Composer', 'Instrumental', 'Score', 300)
    expect(result?.syncedLyrics).toBeNull()
    expect(result?.plainLyrics).toBeNull()
    expect(result?.instrumental).toBe(true)
  })
})
