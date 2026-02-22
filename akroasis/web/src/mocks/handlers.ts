// Mock API request handlers
import { http, HttpResponse, delay } from 'msw'
import { mockArtists, mockAlbums, mockTracks } from './data'
import { audiobookHandlers } from './audiobook-handlers'
import {
  mockUser, mockSessions, mockHistoryEntries, mockPodcastShows, mockPodcastEpisodes,
} from './api-data'

const BASE_URL = 'http://localhost:5000'

export const handlers = [
  // --- Auth ---

  http.post(`${BASE_URL}/api/v3/auth/login`, async () => {
    await delay(500)
    return HttpResponse.json({
      accessToken: 'mock-access-token-12345',
      refreshToken: 'mock-refresh-token-67890',
      user: mockUser,
    })
  }),

  http.post(`${BASE_URL}/api/v3/auth/refresh`, async () => {
    await delay(200)
    return HttpResponse.json({
      accessToken: 'mock-access-token-refreshed',
      refreshToken: 'mock-refresh-token-refreshed',
      user: mockUser,
    })
  }),

  http.post(`${BASE_URL}/api/v3/auth/logout`, async () => {
    await delay(100)
    return new HttpResponse(null, { status: 204 })
  }),

  // --- Music ---

  http.get(`${BASE_URL}/api/v3/artists`, async () => {
    await delay(200)
    return HttpResponse.json(mockArtists)
  }),

  http.get(`${BASE_URL}/api/v3/artists/:id/albums`, async ({ params }) => {
    await delay(200)
    const artistId = Number(params.id)
    const albums = mockAlbums.filter(
      (album) => mockArtists.find((a) => a.id === artistId)?.name === album.artist
    )
    return HttpResponse.json(albums)
  }),

  http.get(`${BASE_URL}/api/v3/albums`, async () => {
    await delay(200)
    return HttpResponse.json(mockAlbums)
  }),

  http.get(`${BASE_URL}/api/v3/albums/:id/tracks`, async ({ params }) => {
    await delay(200)
    const albumId = Number(params.id)
    const album = mockAlbums.find((a) => a.id === albumId)
    const tracks = mockTracks.filter((track) => track.album === album?.title)
    return HttpResponse.json(tracks)
  }),

  http.get(`${BASE_URL}/api/v3/tracks`, async () => {
    await delay(200)
    return HttpResponse.json(mockTracks)
  }),

  http.get(`${BASE_URL}/api/v3/tracks/:id`, async ({ params }) => {
    await delay(200)
    const track = mockTracks.find((t) => t.id === Number(params.id))
    if (!track) {
      return new HttpResponse(null, { status: 404 })
    }
    return HttpResponse.json(track)
  }),

  http.get(`${BASE_URL}/api/v3/stream/:id`, async () => {
    await delay(100)
    return HttpResponse.text('mock-audio-stream', {
      headers: {
        'Content-Type': 'audio/flac',
        'Accept-Ranges': 'bytes',
      },
    })
  }),

  http.get(`${BASE_URL}/api/v3/mediacover/track/:id/poster.jpg`, async () => {
    return HttpResponse.redirect('https://picsum.photos/400', 302)
  }),

  // --- Search ---

  http.get(`${BASE_URL}/api/v3/search`, async ({ request }) => {
    await delay(100)
    const url = new URL(request.url)
    const q = url.searchParams.get('q')?.toLowerCase() ?? ''
    const limit = Number(url.searchParams.get('limit') ?? '50')
    const results = mockTracks
      .filter((t) =>
        t.title.toLowerCase().includes(q) ||
        t.artist.toLowerCase().includes(q) ||
        t.album.toLowerCase().includes(q)
      )
      .slice(0, limit)
      .map((t) => ({
        trackId: t.id,
        title: t.title,
        artist: t.artist,
        album: t.album,
        trackNumber: 1,
        discNumber: 1,
        durationSeconds: Math.floor(t.duration / 1000),
        lossless: t.format.toUpperCase() === 'FLAC',
        relevanceScore: 1.0,
      }))
    return HttpResponse.json(results)
  }),

  // --- Sessions ---

  http.get(`${BASE_URL}/api/v3/sessions`, async () => {
    await delay(200)
    return HttpResponse.json(mockSessions)
  }),

  http.get(`${BASE_URL}/api/v3/sessions/:sessionId`, async ({ params }) => {
    await delay(100)
    const session = mockSessions.find((s) => s.sessionId === params.sessionId)
    if (!session) return new HttpResponse(null, { status: 404 })
    return HttpResponse.json(session)
  }),

  http.get(`${BASE_URL}/api/v3/sessions/media/:mediaItemId`, async ({ params }) => {
    await delay(200)
    const sessions = mockSessions.filter((s) => s.mediaItemId === Number(params.mediaItemId))
    return HttpResponse.json(sessions)
  }),

  http.post(`${BASE_URL}/api/v3/sessions`, async ({ request }) => {
    await delay(100)
    const body = await request.json() as Record<string, unknown>
    return HttpResponse.json({ id: 100, ...body }, { status: 201 })
  }),

  http.put(`${BASE_URL}/api/v3/sessions/:sessionId`, async ({ request, params }) => {
    await delay(100)
    const body = await request.json() as Record<string, unknown>
    const existing = mockSessions.find((s) => s.sessionId === params.sessionId)
    return HttpResponse.json({ ...existing, ...body })
  }),

  http.delete(`${BASE_URL}/api/v3/sessions/:sessionId`, async () => {
    return new HttpResponse(null, { status: 204 })
  }),

  // --- History ---

  http.get(`${BASE_URL}/api/v3/history`, async ({ request }) => {
    await delay(200)
    const url = new URL(request.url)
    const page = Number(url.searchParams.get('page') ?? '1')
    const pageSize = Number(url.searchParams.get('pageSize') ?? '50')
    const start = (page - 1) * pageSize
    const records = mockHistoryEntries.slice(start, start + pageSize)
    return HttpResponse.json({
      page,
      pageSize,
      totalRecords: mockHistoryEntries.length,
      records,
    })
  }),

  http.get(`${BASE_URL}/api/v3/history/since`, async () => {
    await delay(200)
    return HttpResponse.json(mockHistoryEntries)
  }),

  http.get(`${BASE_URL}/api/v3/history/mediaitem/:id`, async ({ params }) => {
    await delay(200)
    const entries = mockHistoryEntries.filter((e) => e.mediaItemId === Number(params.id))
    return HttpResponse.json(entries)
  }),

  http.post(`${BASE_URL}/api/v3/history`, async ({ request }) => {
    await delay(100)
    const body = await request.json() as Record<string, unknown>
    return HttpResponse.json({ id: 200, ...body }, { status: 201 })
  }),

  http.delete(`${BASE_URL}/api/v3/history/:id`, async () => {
    return new HttpResponse(null, { status: 204 })
  }),

  // --- Podcasts ---

  http.get(`${BASE_URL}/api/v3/podcasts`, async ({ request }) => {
    await delay(200)
    const url = new URL(request.url)
    const page = Number(url.searchParams.get('page') ?? '1')
    const pageSize = Number(url.searchParams.get('pageSize') ?? '50')
    const start = (page - 1) * pageSize
    const items = mockPodcastShows.slice(start, start + pageSize)
    return HttpResponse.json({
      items,
      page,
      pageSize,
      totalCount: mockPodcastShows.length,
    })
  }),

  http.get(`${BASE_URL}/api/v3/podcasts/:id/episodes`, async ({ params }) => {
    await delay(200)
    const episodes = mockPodcastEpisodes.filter((e) => e.podcastShowId === Number(params.id))
    return HttpResponse.json(episodes)
  }),

  http.get(`${BASE_URL}/api/v3/podcasts/episodes/:episodeId`, async ({ params }) => {
    await delay(100)
    const episode = mockPodcastEpisodes.find((e) => e.id === Number(params.episodeId))
    if (!episode) return new HttpResponse(null, { status: 404 })
    return HttpResponse.json(episode)
  }),

  http.get(`${BASE_URL}/api/v3/podcasts/:id`, async ({ params }) => {
    await delay(200)
    const show = mockPodcastShows.find((s) => s.id === Number(params.id))
    if (!show) return new HttpResponse(null, { status: 404 })
    return HttpResponse.json(show)
  }),

  http.post(`${BASE_URL}/api/v3/podcasts`, async ({ request }) => {
    await delay(100)
    const body = await request.json() as Record<string, unknown>
    return HttpResponse.json({ id: 300, added: new Date().toISOString(), ...body }, { status: 201 })
  }),

  http.put(`${BASE_URL}/api/v3/podcasts/:id`, async ({ request, params }) => {
    await delay(100)
    const body = await request.json() as Record<string, unknown>
    const existing = mockPodcastShows.find((s) => s.id === Number(params.id))
    return HttpResponse.json({ ...existing, ...body })
  }),

  http.delete(`${BASE_URL}/api/v3/podcasts/:id`, async () => {
    return new HttpResponse(null, { status: 204 })
  }),

  // Audiobook handlers
  ...audiobookHandlers,
]
