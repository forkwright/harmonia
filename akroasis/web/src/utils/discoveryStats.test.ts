import { describe, it, expect, vi, afterEach } from 'vitest'
import type { Track, PlaybackSession } from '../types'
import {
  buildTrackIndex,
  buildPlayRecords,
  computeRediscoverCandidates,
  computeOnThisDay,
  computeDailyActivity,
  computeListeningStats,
  computeTopTracks,
  computeTopArtists,
  computeTopAlbums,
  computeYearInReview,
  computeNewForYou,
  computeListeningDna,
} from './discoveryStats'

const mockTrack = (id: number, title: string, artist: string, album: string): Track => ({
  id, title, artist, album,
  duration: 240, fileSize: 5000000, format: 'FLAC',
  bitrate: 1411, sampleRate: 44100, bitDepth: 16, channels: 2,
})

const mockSession = (
  id: number,
  mediaItemId: number,
  startedAt: string,
  durationMs: number,
): PlaybackSession => ({
  id, sessionId: `session-${id}`, mediaItemId, userId: 'default',
  deviceName: 'Chrome', deviceType: 'desktop', startedAt,
  startPositionMs: 0, durationMs, isActive: false,
})

const tracks = [
  mockTrack(1, 'Song A', 'Artist One', 'Album X'),
  mockTrack(2, 'Song B', 'Artist One', 'Album X'),
  mockTrack(3, 'Song C', 'Artist Two', 'Album Y'),
  mockTrack(4, 'Song D', 'Artist Three', 'Album Z'),
]

describe('buildTrackIndex', () => {
  it('builds map from track array', () => {
    const index = buildTrackIndex(tracks)
    expect(index.size).toBe(4)
    expect(index.get(1)?.title).toBe('Song A')
    expect(index.get(4)?.artist).toBe('Artist Three')
  })

  it('handles empty array', () => {
    expect(buildTrackIndex([]).size).toBe(0)
  })

  it('last track wins on duplicate id', () => {
    const dupes = [mockTrack(1, 'First', 'A', 'X'), mockTrack(1, 'Second', 'A', 'X')]
    const index = buildTrackIndex(dupes)
    expect(index.get(1)?.title).toBe('Second')
  })
})

describe('buildPlayRecords', () => {
  it('aggregates sessions by mediaItemId', () => {
    const sessions = [
      mockSession(1, 10, '2026-01-15T10:00:00Z', 60000),
      mockSession(2, 10, '2026-02-15T10:00:00Z', 120000),
      mockSession(3, 20, '2026-01-20T10:00:00Z', 30000),
    ]

    const records = buildPlayRecords(sessions)

    expect(records.size).toBe(2)

    const r10 = records.get(10)!
    expect(r10.playCount).toBe(2)
    expect(r10.totalDurationMs).toBe(180000)
    expect(r10.lastPlayedAt.toISOString()).toContain('2026-02-15')
    expect(r10.firstPlayedAt.toISOString()).toContain('2026-01-15')

    const r20 = records.get(20)!
    expect(r20.playCount).toBe(1)
  })

  it('handles empty sessions', () => {
    expect(buildPlayRecords([]).size).toBe(0)
  })

  it('single session per track', () => {
    const sessions = [mockSession(1, 5, '2026-01-01T00:00:00Z', 50000)]
    const records = buildPlayRecords(sessions)
    const r = records.get(5)!
    expect(r.playCount).toBe(1)
    expect(r.firstPlayedAt.getTime()).toBe(r.lastPlayedAt.getTime())
  })
})

describe('computeRediscoverCandidates', () => {
  afterEach(() => vi.restoreAllMocks())

  it('returns tracks with 2+ plays and last played > threshold', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-08-01T00:00:00Z'))

    const sessions = [
      mockSession(1, 1, '2026-01-01T00:00:00Z', 60000),
      mockSession(2, 1, '2026-01-10T00:00:00Z', 60000),
      mockSession(3, 2, '2026-07-20T00:00:00Z', 60000),
      mockSession(4, 2, '2026-07-25T00:00:00Z', 60000),
    ]

    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)
    const candidates = computeRediscoverCandidates(records, index)

    expect(candidates).toHaveLength(1)
    expect(candidates[0].track.id).toBe(1)
    expect(candidates[0].playCount).toBe(2)

    vi.useRealTimers()
  })

  it('excludes tracks with only 1 play', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-08-01T00:00:00Z'))

    const sessions = [mockSession(1, 1, '2026-01-01T00:00:00Z', 60000)]
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)

    expect(computeRediscoverCandidates(records, index)).toHaveLength(0)

    vi.useRealTimers()
  })

  it('excludes tracks with recent plays', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-08-01T00:00:00Z'))

    const sessions = [
      mockSession(1, 1, '2026-07-20T00:00:00Z', 60000),
      mockSession(2, 1, '2026-07-25T00:00:00Z', 60000),
    ]
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)

    expect(computeRediscoverCandidates(records, index)).toHaveLength(0)

    vi.useRealTimers()
  })

  it('handles missing tracks in index', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-08-01T00:00:00Z'))

    const sessions = [
      mockSession(1, 999, '2026-01-01T00:00:00Z', 60000),
      mockSession(2, 999, '2026-01-10T00:00:00Z', 60000),
    ]
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)

    expect(computeRediscoverCandidates(records, index)).toHaveLength(0)

    vi.useRealTimers()
  })

  it('respects configurable threshold', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-04-01T00:00:00Z'))

    const sessions = [
      mockSession(1, 1, '2026-01-01T00:00:00Z', 60000),
      mockSession(2, 1, '2026-01-10T00:00:00Z', 60000),
    ]
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)

    expect(computeRediscoverCandidates(records, index, 6)).toHaveLength(0)
    expect(computeRediscoverCandidates(records, index, 2)).toHaveLength(1)

    vi.useRealTimers()
  })

  it('sorts by play count descending', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-08-01T00:00:00Z'))

    const sessions = [
      mockSession(1, 1, '2026-01-01T00:00:00Z', 60000),
      mockSession(2, 1, '2026-01-02T00:00:00Z', 60000),
      mockSession(3, 3, '2026-01-01T00:00:00Z', 60000),
      mockSession(4, 3, '2026-01-02T00:00:00Z', 60000),
      mockSession(5, 3, '2026-01-03T00:00:00Z', 60000),
    ]
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)
    const candidates = computeRediscoverCandidates(records, index)

    expect(candidates[0].track.id).toBe(3)
    expect(candidates[0].playCount).toBe(3)
    expect(candidates[1].track.id).toBe(1)

    vi.useRealTimers()
  })

  it('returns empty for empty input', () => {
    expect(computeRediscoverCandidates(new Map(), new Map())).toEqual([])
  })
})

describe('computeOnThisDay', () => {
  afterEach(() => vi.restoreAllMocks())

  it('matches sessions by month+day, excludes current year', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-02-21T12:00:00Z'))

    const sessions = [
      mockSession(1, 1, '2025-02-21T10:00:00Z', 60000),
      mockSession(2, 2, '2024-02-21T15:00:00Z', 120000),
      mockSession(3, 3, '2026-02-21T10:00:00Z', 60000), // current year — excluded
      mockSession(4, 4, '2025-03-21T10:00:00Z', 60000), // wrong month
    ]

    const index = buildTrackIndex(tracks)
    const result = computeOnThisDay(sessions, index)

    expect(result).toHaveLength(2)
    expect(result[0].session.id).toBe(1) // more recent year first
    expect(result[0].track?.title).toBe('Song A')
    expect(result[1].session.id).toBe(2)

    vi.useRealTimers()
  })

  it('handles missing track gracefully', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-02-21T12:00:00Z'))

    const sessions = [mockSession(1, 999, '2025-02-21T10:00:00Z', 60000)]
    const result = computeOnThisDay(sessions, buildTrackIndex(tracks))

    expect(result).toHaveLength(1)
    expect(result[0].track).toBeUndefined()

    vi.useRealTimers()
  })

  it('returns empty when no matches', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-02-21T12:00:00Z'))

    const sessions = [mockSession(1, 1, '2025-03-15T10:00:00Z', 60000)]
    expect(computeOnThisDay(sessions, buildTrackIndex(tracks))).toHaveLength(0)

    vi.useRealTimers()
  })
})

describe('computeDailyActivity', () => {
  afterEach(() => vi.restoreAllMocks())

  it('produces 364 entries', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-02-21T12:00:00Z'))

    const result = computeDailyActivity([])
    expect(result).toHaveLength(364)

    vi.useRealTimers()
  })

  it('correctly aggregates multiple sessions on same day', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-02-21T12:00:00Z'))

    const sessions = [
      mockSession(1, 1, '2026-02-20T10:00:00Z', 1800000), // 30 min
      mockSession(2, 2, '2026-02-20T14:00:00Z', 2700000), // 45 min
    ]

    const result = computeDailyActivity(sessions)
    const feb20 = result.find((d) => d.date === '2026-02-20')
    expect(feb20?.durationMinutes).toBe(75) // 30 + 45

    vi.useRealTimers()
  })

  it('handles empty input', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-02-21T12:00:00Z'))

    const result = computeDailyActivity([])
    expect(result.every((d) => d.durationMinutes === 0)).toBe(true)

    vi.useRealTimers()
  })

  it('ignores sessions older than 52 weeks', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-02-21T12:00:00Z'))

    const sessions = [
      mockSession(1, 1, '2024-01-01T10:00:00Z', 3600000),
    ]

    const result = computeDailyActivity(sessions)
    expect(result.every((d) => d.durationMinutes === 0)).toBe(true)

    vi.useRealTimers()
  })
})

describe('computeListeningStats', () => {
  afterEach(() => vi.restoreAllMocks())

  it('calculates all stats correctly', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-02-21T12:00:00Z'))

    const sessions = [
      mockSession(1, 1, '2026-02-21T08:00:00Z', 3600000),  // today: 1h
      mockSession(2, 2, '2026-02-20T10:00:00Z', 1800000),  // this week: 30m
      mockSession(3, 3, '2026-02-10T10:00:00Z', 7200000),  // this month: 2h
      mockSession(4, 4, '2025-12-01T10:00:00Z', 600000),   // older: 10m
    ]

    const stats = computeListeningStats(sessions)

    expect(stats.todayMs).toBe(3600000)
    expect(stats.thisWeekMs).toBe(3600000 + 1800000) // today + yesterday (Fri)
    expect(stats.thisMonthMs).toBe(3600000 + 1800000 + 7200000)
    expect(stats.allTimeMs).toBe(3600000 + 1800000 + 7200000 + 600000)
    expect(stats.totalSessions).toBe(4)
    expect(stats.activeDays).toBe(4)
  })

  it('handles empty sessions', () => {
    const stats = computeListeningStats([])
    expect(stats.allTimeMs).toBe(0)
    expect(stats.avgDailyMs).toBe(0)
    expect(stats.activeDays).toBe(0)
  })
})

describe('computeTopTracks', () => {
  it('ranks by play count', () => {
    const sessions = [
      mockSession(1, 1, '2026-02-01T10:00:00Z', 60000),
      mockSession(2, 3, '2026-02-01T11:00:00Z', 60000),
      mockSession(3, 3, '2026-02-02T11:00:00Z', 60000),
      mockSession(4, 3, '2026-02-03T11:00:00Z', 60000),
      mockSession(5, 1, '2026-02-02T10:00:00Z', 60000),
    ]

    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)
    const top = computeTopTracks(records, index)

    expect(top[0].name).toContain('Song C')
    expect(top[0].count).toBe(3)
    expect(top[1].name).toContain('Song A')
    expect(top[1].count).toBe(2)
  })

  it('respects limit', () => {
    const sessions = tracks.map((t, i) =>
      mockSession(i + 1, t.id, `2026-02-0${i + 1}T10:00:00Z`, 60000),
    )
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)

    expect(computeTopTracks(records, index, 2)).toHaveLength(2)
  })

  it('handles missing track data', () => {
    const sessions = [mockSession(1, 999, '2026-02-01T10:00:00Z', 60000)]
    const records = buildPlayRecords(sessions)
    const top = computeTopTracks(records, buildTrackIndex(tracks))
    expect(top).toHaveLength(0)
  })
})

describe('computeTopArtists', () => {
  it('aggregates across tracks from same artist', () => {
    const sessions = [
      mockSession(1, 1, '2026-02-01T10:00:00Z', 60000),
      mockSession(2, 2, '2026-02-01T11:00:00Z', 60000),
      mockSession(3, 3, '2026-02-01T12:00:00Z', 60000),
    ]

    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)
    const top = computeTopArtists(records, index)

    expect(top[0].name).toBe('Artist One')
    expect(top[0].count).toBe(2) // tracks 1 + 2
    expect(top[1].name).toBe('Artist Two')
    expect(top[1].count).toBe(1)
  })

  it('respects limit', () => {
    const sessions = tracks.map((t, i) =>
      mockSession(i + 1, t.id, `2026-02-0${i + 1}T10:00:00Z`, 60000),
    )
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)

    expect(computeTopArtists(records, index, 1)).toHaveLength(1)
  })
})

describe('computeTopAlbums', () => {
  it('aggregates across tracks from same album', () => {
    const sessions = [
      mockSession(1, 1, '2026-02-01T10:00:00Z', 60000),
      mockSession(2, 2, '2026-02-01T11:00:00Z', 120000),
      mockSession(3, 3, '2026-02-01T12:00:00Z', 60000),
    ]

    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)
    const top = computeTopAlbums(records, index)

    expect(top[0].name).toContain('Album X')
    expect(top[0].count).toBe(2) // tracks 1 + 2
    expect(top[0].durationMs).toBe(180000) // 60k + 120k
  })

  it('respects limit', () => {
    const sessions = tracks.map((t, i) =>
      mockSession(i + 1, t.id, `2026-02-0${i + 1}T10:00:00Z`, 60000),
    )
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)

    expect(computeTopAlbums(records, index, 1)).toHaveLength(1)
  })
})

describe('computeYearInReview', () => {
  afterEach(() => vi.restoreAllMocks())

  it('aggregates stats for the specified year', () => {
    const sessions = [
      mockSession(1, 1, '2026-01-15T10:00:00Z', 3600000),
      mockSession(2, 2, '2026-03-20T10:00:00Z', 1800000),
      mockSession(3, 3, '2026-03-20T14:00:00Z', 900000),
      mockSession(4, 4, '2025-06-01T10:00:00Z', 7200000), // different year
    ]

    const index = buildTrackIndex(tracks)
    const review = computeYearInReview(sessions, index, 2026)

    expect(review.year).toBe(2026)
    expect(review.totalMs).toBe(3600000 + 1800000 + 900000)
    expect(review.totalSessions).toBe(3)
    expect(review.activeDays).toBe(2) // Jan 15 + Mar 20
    expect(review.topTracks.length).toBeGreaterThan(0)
    expect(review.topArtists.length).toBeGreaterThan(0)
  })

  it('computes monthly breakdown', () => {
    const sessions = [
      mockSession(1, 1, '2026-01-10T10:00:00Z', 3600000),
      mockSession(2, 2, '2026-01-20T10:00:00Z', 1800000),
      mockSession(3, 3, '2026-06-15T10:00:00Z', 7200000),
    ]

    const index = buildTrackIndex(tracks)
    const review = computeYearInReview(sessions, index, 2026)

    expect(review.monthlyBreakdown).toHaveLength(12)
    expect(review.monthlyBreakdown[0].durationMs).toBe(5400000) // Jan
    expect(review.monthlyBreakdown[0].sessions).toBe(2)
    expect(review.monthlyBreakdown[5].durationMs).toBe(7200000) // Jun
    expect(review.monthlyBreakdown[2].durationMs).toBe(0) // Mar — empty
  })

  it('finds most active month', () => {
    const sessions = [
      mockSession(1, 1, '2026-01-10T10:00:00Z', 1000000),
      mockSession(2, 2, '2026-06-15T10:00:00Z', 5000000),
      mockSession(3, 3, '2026-06-20T10:00:00Z', 3000000),
    ]

    const index = buildTrackIndex(tracks)
    const review = computeYearInReview(sessions, index, 2026)

    expect(review.mostActiveMonth).toEqual({ month: 5, durationMs: 8000000 })
  })

  it('returns empty for year with no sessions', () => {
    const sessions = [mockSession(1, 1, '2025-01-10T10:00:00Z', 3600000)]

    const review = computeYearInReview(sessions, buildTrackIndex(tracks), 2026)

    expect(review.totalMs).toBe(0)
    expect(review.totalSessions).toBe(0)
    expect(review.activeDays).toBe(0)
    expect(review.mostActiveMonth).toBeNull()
    expect(review.topTracks).toEqual([])
  })

  it('defaults to previous year in January', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-01-15T12:00:00Z'))

    const sessions = [
      mockSession(1, 1, '2025-06-01T10:00:00Z', 3600000),
    ]

    const review = computeYearInReview(sessions, buildTrackIndex(tracks))
    expect(review.year).toBe(2025)

    vi.useRealTimers()
  })
})

describe('computeNewForYou', () => {
  it('returns empty for no sessions', () => {
    const records = buildPlayRecords([])
    const index = buildTrackIndex(tracks)
    expect(computeNewForYou(records, index, tracks)).toEqual([])
  })

  it('returns empty when all tracks are played', () => {
    const sessions = tracks.map((t, i) =>
      mockSession(i + 1, t.id, `2026-02-0${i + 1}T10:00:00Z`, 60000),
    )
    const doubleSessions = [
      ...sessions,
      ...tracks.map((t, i) => mockSession(i + 10, t.id, `2026-02-0${i + 1}T12:00:00Z`, 60000)),
    ]
    const records = buildPlayRecords(doubleSessions)
    const index = buildTrackIndex(tracks)
    expect(computeNewForYou(records, index, tracks)).toEqual([])
  })

  it('identifies unheard tracks from favorite artists', () => {
    const allTracks = [
      ...tracks,
      mockTrack(5, 'Song E', 'Artist One', 'Album W'),
    ]
    const sessions = [
      mockSession(1, 1, '2026-02-01T10:00:00Z', 60000),
      mockSession(2, 1, '2026-02-02T10:00:00Z', 60000),
      mockSession(3, 2, '2026-02-01T11:00:00Z', 60000),
    ]
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(allTracks)
    const results = computeNewForYou(records, index, allTracks)

    const trackIds = results.map((r) => r.track.id)
    expect(trackIds).toContain(5)
    expect(trackIds).not.toContain(1)
    expect(trackIds).not.toContain(2)
  })

  it('excludes artists with fewer than 2 plays', () => {
    const sessions = [
      mockSession(1, 3, '2026-02-01T10:00:00Z', 60000),
    ]
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)
    expect(computeNewForYou(records, index, tracks)).toEqual([])
  })

  it('identifies incomplete albums', () => {
    const sessions = [
      mockSession(1, 1, '2026-02-01T10:00:00Z', 60000),
      mockSession(2, 1, '2026-02-02T10:00:00Z', 60000),
    ]
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(tracks)
    const results = computeNewForYou(records, index, tracks)

    const track2Result = results.find((r) => r.track.id === 2)
    expect(track2Result).toBeDefined()
    expect(track2Result?.reason).toBe('incomplete_album')
    expect(track2Result?.albumCompletionPct).toBe(50)
  })

  it('sorts incomplete albums before unheard artist tracks', () => {
    const allTracks = [
      ...tracks,
      mockTrack(5, 'Song E', 'Artist One', 'Album W'),
    ]
    const sessions = [
      mockSession(1, 1, '2026-02-01T10:00:00Z', 60000),
      mockSession(2, 1, '2026-02-02T10:00:00Z', 60000),
    ]
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(allTracks)
    const results = computeNewForYou(records, index, allTracks)

    const incompleteIdx = results.findIndex((r) => r.reason === 'incomplete_album')
    const unheardIdx = results.findIndex((r) => r.reason === 'unheard_from_favorite_artist')
    if (incompleteIdx >= 0 && unheardIdx >= 0) {
      expect(incompleteIdx).toBeLessThan(unheardIdx)
    }
  })

  it('respects limit', () => {
    const manyTracks = Array.from({ length: 30 }, (_, i) =>
      mockTrack(100 + i, `Extra ${i}`, 'Artist One', `Album ${i}`),
    )
    const allTracks = [...tracks, ...manyTracks]
    const sessions = [
      mockSession(1, 1, '2026-02-01T10:00:00Z', 60000),
      mockSession(2, 1, '2026-02-02T10:00:00Z', 60000),
    ]
    const records = buildPlayRecords(sessions)
    const index = buildTrackIndex(allTracks)

    expect(computeNewForYou(records, index, allTracks, 5)).toHaveLength(5)
  })
})

describe('computeListeningDna', () => {
  const dnaTracks = [
    mockTrack(1, 'Song A', 'Artist One', 'Album X'),
    mockTrack(2, 'Song B', 'Artist One', 'Album X'),
    mockTrack(3, 'Song C', 'Artist Two', 'Album Y'),
    mockTrack(4, 'Song D', 'Artist Three', 'Album Z'),
    { ...mockTrack(5, 'Song E', 'Artist Four', 'Album W'), format: 'mp3' },
  ]

  const dnaSessions = [
    mockSession(1, 1, '2026-02-01T10:00:00Z', 1200000), // 20min
    mockSession(2, 2, '2026-02-01T14:00:00Z', 900000),  // 15min
    mockSession(3, 3, '2026-02-02T10:00:00Z', 600000),  // 10min
    mockSession(4, 4, '2026-02-03T22:00:00Z', 1800000), // 30min
    mockSession(5, 5, '2026-02-04T10:00:00Z', 300000),  // 5min
    mockSession(6, 1, '2026-02-05T10:00:00Z', 1200000), // repeat
  ]

  function buildDna() {
    const index = buildTrackIndex(dnaTracks)
    const records = buildPlayRecords(dnaSessions)
    return computeListeningDna(dnaSessions, records, index)
  }

  it('computes artist diversity', () => {
    const dna = buildDna()
    expect(dna.artistDiversity.uniqueArtists).toBe(4)
    expect(dna.artistDiversity.totalPlays).toBe(6)
    expect(dna.artistDiversity.entropy).toBeGreaterThan(0)
    expect(dna.artistDiversity.label).toBeDefined()
  })

  it('computes album depth', () => {
    const dna = buildDna()
    expect(dna.albumDepth.albumsStarted).toBeGreaterThan(0)
    expect(dna.albumDepth.completionRate).toBeGreaterThanOrEqual(0)
    expect(dna.albumDepth.label).toBeDefined()
  })

  it('computes album completion — Album X is complete', () => {
    const dna = buildDna()
    // Album X has tracks 1 and 2, both played
    expect(dna.albumDepth.albumsCompleted).toBeGreaterThanOrEqual(1)
  })

  it('computes session patterns', () => {
    const dna = buildDna()
    expect(dna.sessionPatterns.avgSessionMinutes).toBeGreaterThan(0)
    expect(dna.sessionPatterns.peakHour).toBeGreaterThanOrEqual(0)
    expect(dna.sessionPatterns.peakHour).toBeLessThanOrEqual(23)
    expect(dna.sessionPatterns.peakDay).toBeGreaterThanOrEqual(0)
    expect(dna.sessionPatterns.peakDay).toBeLessThanOrEqual(6)
    expect(dna.sessionPatterns.sessionsPerWeek).toBeGreaterThan(0)
    expect(dna.sessionPatterns.label).toBeDefined()
  })

  it('computes format preferences', () => {
    const dna = buildDna()
    // 5 out of 6 plays are FLAC (tracks 1-4), 1 is mp3 (track 5)
    expect(dna.formatPreferences.losslessPct).toBeGreaterThan(50)
    expect(dna.formatPreferences.dominantFormat.toLowerCase()).toBe('flac')
    expect(dna.formatPreferences.label).toBeDefined()
  })

  it('computes listening velocity as 12-element array', () => {
    const dna = buildDna()
    expect(dna.listeningVelocity.tracksPerWeek).toHaveLength(12)
    expect(['accelerating', 'steady', 'decelerating']).toContain(dna.listeningVelocity.trend)
    expect(dna.listeningVelocity.label).toBeDefined()
  })

  it('handles empty sessions', () => {
    const index = buildTrackIndex(dnaTracks)
    const records = buildPlayRecords([])
    const dna = computeListeningDna([], records, index)
    expect(dna.artistDiversity.totalPlays).toBe(0)
    expect(dna.artistDiversity.entropy).toBe(0)
    expect(dna.albumDepth.albumsStarted).toBe(0)
    expect(dna.sessionPatterns.avgSessionMinutes).toBe(0)
    expect(dna.formatPreferences.losslessPct).toBe(0)
    expect(dna.listeningVelocity.tracksPerWeek).toHaveLength(12)
  })

  it('labels high entropy as Explorer or Curious', () => {
    // Create many diverse artists
    const manyTracks = Array.from({ length: 30 }, (_, i) => mockTrack(i + 100, `T${i}`, `A${i}`, `Alb${i}`))
    const manySessions = manyTracks.map((t, i) => mockSession(i + 100, t.id, `2026-02-${String(i % 28 + 1).padStart(2, '0')}T10:00:00Z`, 300000))
    const idx = buildTrackIndex(manyTracks)
    const rec = buildPlayRecords(manySessions)
    const dna = computeListeningDna(manySessions, rec, idx)
    expect(['Explorer', 'Curious']).toContain(dna.artistDiversity.label)
  })

  it('labels low entropy as Loyalist or Focused', () => {
    // Single artist, many plays
    const t = [mockTrack(200, 'X', 'Solo', 'A')]
    const s = Array.from({ length: 10 }, (_, i) => mockSession(200 + i, 200, `2026-02-01T${String(i + 10).padStart(2, '0')}:00:00Z`, 300000))
    const idx = buildTrackIndex(t)
    const rec = buildPlayRecords(s)
    const dna = computeListeningDna(s, rec, idx)
    expect(['Loyalist', 'Focused']).toContain(dna.artistDiversity.label)
  })
})
