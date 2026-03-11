// Mock API handlers for audiobook endpoints
import { http, HttpResponse, delay } from 'msw'
import { mockAuthors, mockAudiobooks, mockChapters, mockProgress, mockContinueItems } from './audiobook-data'

const BASE_URL = 'http://localhost:5000'

export const audiobookHandlers = [
  // Authors
  http.get(`${BASE_URL}/api/v3/authors`, async ({ request }) => {
    await delay(200)
    const url = new URL(request.url)
    const page = Number(url.searchParams.get('page') ?? '1')
    const pageSize = Number(url.searchParams.get('pageSize') ?? '50')
    const start = (page - 1) * pageSize
    const items = mockAuthors.slice(start, start + pageSize)
    return HttpResponse.json({
      items,
      page,
      pageSize,
      totalCount: mockAuthors.length,
    })
  }),

  http.get(`${BASE_URL}/api/v3/authors/:id`, async ({ params }) => {
    await delay(200)
    const author = mockAuthors.find((a) => a.id === Number(params.id))
    if (!author) return new HttpResponse(null, { status: 404 })
    return HttpResponse.json(author)
  }),

  // Audiobooks
  http.get(`${BASE_URL}/api/v3/audiobooks`, async ({ request }) => {
    await delay(200)
    const url = new URL(request.url)
    const page = Number(url.searchParams.get('page') ?? '1')
    const pageSize = Number(url.searchParams.get('pageSize') ?? '50')
    const start = (page - 1) * pageSize
    const items = mockAudiobooks.slice(start, start + pageSize)
    return HttpResponse.json({
      items,
      page,
      pageSize,
      totalCount: mockAudiobooks.length,
    })
  }),

  http.get(`${BASE_URL}/api/v3/audiobooks/author/:authorId`, async ({ params }) => {
    await delay(200)
    const books = mockAudiobooks.filter((b) => b.authorId === Number(params.authorId))
    return HttpResponse.json(books)
  }),

  http.get(`${BASE_URL}/api/v3/audiobooks/series/:seriesId`, async ({ params }) => {
    await delay(200)
    const books = mockAudiobooks.filter((b) => b.bookSeriesId === Number(params.seriesId))
    return HttpResponse.json(books)
  }),

  http.get(`${BASE_URL}/api/v3/audiobooks/:id`, async ({ params }) => {
    await delay(200)
    const book = mockAudiobooks.find((b) => b.id === Number(params.id))
    if (!book) return new HttpResponse(null, { status: 404 })
    return HttpResponse.json(book)
  }),

  // Chapters
  http.get(`${BASE_URL}/api/v3/chapters/:mediaFileId`, async ({ params }) => {
    await delay(100)
    const chapters = mockChapters[Number(params.mediaFileId)] ?? []
    return HttpResponse.json(chapters)
  }),

  // Progress
  http.get(`${BASE_URL}/api/v3/progress/:mediaItemId`, async ({ params }) => {
    await delay(100)
    const progress = mockProgress[Number(params.mediaItemId)]
    if (!progress) return new HttpResponse(null, { status: 404 })
    return HttpResponse.json(progress)
  }),

  http.post(`${BASE_URL}/api/v3/progress`, async ({ request }) => {
    await delay(100)
    const body = await request.json() as { mediaItemId: number; positionMs: number; totalDurationMs: number }
    return HttpResponse.json({
      id: 99,
      mediaItemId: body.mediaItemId,
      userId: 'default',
      positionMs: body.positionMs,
      totalDurationMs: body.totalDurationMs,
      percentComplete: body.totalDurationMs > 0 ? (body.positionMs / body.totalDurationMs) * 100 : 0,
      lastPlayedAt: new Date().toISOString(),
      isComplete: false,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    })
  }),

  // Continue listening
  http.get(`${BASE_URL}/api/v3/continue`, async () => {
    await delay(200)
    return HttpResponse.json(mockContinueItems)
  }),

]
