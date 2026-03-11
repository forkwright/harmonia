// Mock API request handlers
import { http, HttpResponse, delay } from 'msw'
import { mockArtists, mockAlbums, mockTracks, mockFavoriteIds } from './data'
import { audiobookHandlers } from './audiobook-handlers'
import {
  mockUser, mockSessions, mockHistoryEntries, mockPodcastShows, mockPodcastEpisodes,
  mockPlaylists, mockPlaylistTracks, createMockPlaylist,
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

  http.get(`${BASE_URL}/api/v3/artists/music`, async ({ request }) => {
    await delay(200)
    const url = new URL(request.url)
    const page = Number(url.searchParams.get('page') ?? '1')
    const pageSize = Number(url.searchParams.get('pageSize') ?? '50')
    const start = (page - 1) * pageSize
    const items = mockArtists.slice(start, start + pageSize)
    return HttpResponse.json({
      items,
      page,
      pageSize,
      totalCount: mockArtists.length,
    })
  }),

  http.get(`${BASE_URL}/api/v3/albums/artist/:id`, async ({ params }) => {
    await delay(200)
    const artistId = Number(params.id)
    const albums = mockAlbums.filter(
      (album) => mockArtists.find((a) => a.id === artistId)?.name === album.artist
    )
    return HttpResponse.json(albums)
  }),

  http.get(`${BASE_URL}/api/v3/albums`, async ({ request }) => {
    await delay(200)
    const url = new URL(request.url)
    const page = Number(url.searchParams.get('page') ?? '1')
    const pageSize = Number(url.searchParams.get('pageSize') ?? '50')
    const start = (page - 1) * pageSize
    const items = mockAlbums.slice(start, start + pageSize)
    return HttpResponse.json({
      items,
      page,
      pageSize,
      totalCount: mockAlbums.length,
    })
  }),

  http.get(`${BASE_URL}/api/v3/tracks/album/:id`, async ({ params }) => {
    await delay(200)
    const albumId = Number(params.id)
    const album = mockAlbums.find((a) => a.id === albumId)
    const tracks = mockTracks.filter((track) => track.album === album?.title)
    return HttpResponse.json(tracks)
  }),

  http.get(`${BASE_URL}/api/v3/tracks`, async ({ request }) => {
    await delay(200)
    const url = new URL(request.url)
    const page = Number(url.searchParams.get('page') ?? '1')
    const pageSize = Number(url.searchParams.get('pageSize') ?? '50')
    const start = (page - 1) * pageSize
    const items = mockTracks.slice(start, start + pageSize)
    return HttpResponse.json({
      items,
      page,
      pageSize,
      totalCount: mockTracks.length,
    })
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

  // --- Favorites ---

  http.get(`${BASE_URL}/api/v3/favorites/ids`, async () => {
    await delay(100)
    return HttpResponse.json(mockFavoriteIds)
  }),

  http.get(`${BASE_URL}/api/v3/favorites`, async ({ request }) => {
    await delay(200)
    const url = new URL(request.url)
    const page = Number(url.searchParams.get('page') ?? '1')
    const pageSize = Number(url.searchParams.get('pageSize') ?? '50')
    const favTracks = mockTracks.filter((t) => mockFavoriteIds.includes(t.id))
    const start = (page - 1) * pageSize
    const items = favTracks.slice(start, start + pageSize)
    return HttpResponse.json({
      items,
      page,
      pageSize,
      totalCount: favTracks.length,
    })
  }),

  http.post(`${BASE_URL}/api/v3/favorites/:trackId`, async ({ params }) => {
    await delay(100)
    const trackId = Number(params.trackId)
    if (!mockFavoriteIds.includes(trackId)) {
      mockFavoriteIds.push(trackId)
    }
    return new HttpResponse(null, { status: 201 })
  }),

  http.delete(`${BASE_URL}/api/v3/favorites/:trackId`, async ({ params }) => {
    await delay(100)
    const trackId = Number(params.trackId)
    const idx = mockFavoriteIds.indexOf(trackId)
    if (idx !== -1) mockFavoriteIds.splice(idx, 1)
    return new HttpResponse(null, { status: 204 })
  }),

  // --- Playlists ---

  http.get(`${BASE_URL}/api/v3/playlists`, async ({ request }) => {
    await delay(200)
    const url = new URL(request.url)
    const page = Number(url.searchParams.get('page') ?? '1')
    const pageSize = Number(url.searchParams.get('pageSize') ?? '50')
    const start = (page - 1) * pageSize
    const items = mockPlaylists.slice(start, start + pageSize)
    return HttpResponse.json({ items, page, pageSize, totalCount: mockPlaylists.length })
  }),

  http.get(`${BASE_URL}/api/v3/playlists/:id/tracks`, async ({ params }) => {
    await delay(200)
    const playlistId = Number(params.id)
    const trackIds = mockPlaylistTracks[playlistId] ?? []
    const tracks = mockTracks.filter((t) => trackIds.includes(t.id))
    return HttpResponse.json(tracks)
  }),

  http.post(`${BASE_URL}/api/v3/playlists`, async ({ request }) => {
    await delay(100)
    const body = await request.json() as { name: string; description?: string }
    const playlist = createMockPlaylist(body.name, body.description)
    return HttpResponse.json(playlist, { status: 201 })
  }),

  http.put(`${BASE_URL}/api/v3/playlists/:id`, async ({ request, params }) => {
    await delay(100)
    const body = await request.json() as Record<string, unknown>
    const existing = mockPlaylists.find((p) => p.id === Number(params.id))
    if (!existing) return new HttpResponse(null, { status: 404 })
    Object.assign(existing, body, { updatedAt: new Date().toISOString() })
    return HttpResponse.json(existing)
  }),

  http.delete(`${BASE_URL}/api/v3/playlists/:id`, async ({ params }) => {
    const idx = mockPlaylists.findIndex((p) => p.id === Number(params.id))
    if (idx !== -1) mockPlaylists.splice(idx, 1)
    delete mockPlaylistTracks[Number(params.id)]
    return new HttpResponse(null, { status: 204 })
  }),

  http.post(`${BASE_URL}/api/v3/playlists/:id/tracks`, async ({ request, params }) => {
    await delay(100)
    const body = await request.json() as { trackId: number }
    const playlistId = Number(params.id)
    if (!mockPlaylistTracks[playlistId]) mockPlaylistTracks[playlistId] = []
    mockPlaylistTracks[playlistId].push(body.trackId)
    const playlist = mockPlaylists.find((p) => p.id === playlistId)
    if (playlist) playlist.trackCount++
    return new HttpResponse(null, { status: 201 })
  }),

  http.delete(`${BASE_URL}/api/v3/playlists/:playlistId/tracks/:trackId`, async ({ params }) => {
    const playlistId = Number(params.playlistId)
    const trackId = Number(params.trackId)
    const trackIds = mockPlaylistTracks[playlistId]
    if (trackIds) {
      const idx = trackIds.indexOf(trackId)
      if (idx !== -1) trackIds.splice(idx, 1)
    }
    const playlist = mockPlaylists.find((p) => p.id === playlistId)
    if (playlist) playlist.trackCount = Math.max(0, playlist.trackCount - 1)
    return new HttpResponse(null, { status: 204 })
  }),

  http.put(`${BASE_URL}/api/v3/playlists/:id/tracks/reorder`, async ({ request, params }) => {
    await delay(100)
    const body = await request.json() as { trackIds: number[] }
    mockPlaylistTracks[Number(params.id)] = body.trackIds
    return new HttpResponse(null, { status: 204 })
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
    return HttpResponse.json(results.slice(0, limit))
  }),

  // --- Scrobble ---

  http.post(`${BASE_URL}/api/v1/scrobble`, async () => {
    await delay(100)
    return new HttpResponse(null, { status: 204 })
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
