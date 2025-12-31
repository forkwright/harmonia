// Mock API request handlers
import { http, HttpResponse, delay } from 'msw'
import { mockArtists, mockAlbums, mockTracks } from './data'

const BASE_URL = 'http://localhost:5000'

export const handlers = [
  http.post(`${BASE_URL}/api/v3/auth/login`, async () => {
    await delay(500)
    return HttpResponse.json({
      token: 'mock-api-key-12345',
      expiresIn: 86400,
    })
  }),

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
]
