// Pure computation functions for discovery intelligence
import type { Track, PlaybackSession } from '../types'

export interface TrackPlayRecord {
  mediaItemId: number
  playCount: number
  lastPlayedAt: Date
  firstPlayedAt: Date
  totalDurationMs: number
}

export interface EnrichedSession {
  session: PlaybackSession
  track?: Track
}

export interface DayActivity {
  date: string
  durationMinutes: number
}

export interface TopItem {
  name: string
  count: number
  durationMs: number
  coverArtUrl?: string
  id?: number
}

export interface ListeningStats {
  allTimeMs: number
  thisMonthMs: number
  thisWeekMs: number
  todayMs: number
  avgDailyMs: number
  totalSessions: number
  activeDays: number
}

export interface NewForYouItem {
  track: Track
  reason: 'unheard_from_favorite_artist' | 'incomplete_album'
  artistPlayCount: number
  albumCompletionPct?: number
}

export interface YearInReview {
  year: number
  totalMs: number
  totalSessions: number
  activeDays: number
  topTracks: TopItem[]
  topArtists: TopItem[]
  topAlbums: TopItem[]
  mostActiveMonth: { month: number; durationMs: number } | null
  monthlyBreakdown: Array<{ month: number; durationMs: number; sessions: number }>
}

export function buildTrackIndex(tracks: Track[]): Map<number, Track> {
  const index = new Map<number, Track>()
  for (const track of tracks) {
    index.set(track.id, track)
  }
  return index
}

export function buildPlayRecords(sessions: PlaybackSession[]): Map<number, TrackPlayRecord> {
  const records = new Map<number, TrackPlayRecord>()

  for (const session of sessions) {
    const existing = records.get(session.mediaItemId)
    const startDate = new Date(session.startedAt)

    if (existing) {
      existing.playCount++
      existing.totalDurationMs += session.durationMs
      if (startDate > existing.lastPlayedAt) existing.lastPlayedAt = startDate
      if (startDate < existing.firstPlayedAt) existing.firstPlayedAt = startDate
    } else {
      records.set(session.mediaItemId, {
        mediaItemId: session.mediaItemId,
        playCount: 1,
        lastPlayedAt: startDate,
        firstPlayedAt: startDate,
        totalDurationMs: session.durationMs,
      })
    }
  }

  return records
}

export function computeRediscoverCandidates(
  playRecords: Map<number, TrackPlayRecord>,
  trackIndex: Map<number, Track>,
  thresholdMonths = 6,
): Array<{ track: Track; playCount: number; lastPlayed: Date }> {
  const threshold = new Date()
  threshold.setMonth(threshold.getMonth() - thresholdMonths)

  const candidates: Array<{ track: Track; playCount: number; lastPlayed: Date }> = []

  for (const [mediaItemId, record] of playRecords) {
    if (record.playCount < 2) continue
    if (record.lastPlayedAt >= threshold) continue

    const track = trackIndex.get(mediaItemId)
    if (!track) continue

    candidates.push({
      track,
      playCount: record.playCount,
      lastPlayed: record.lastPlayedAt,
    })
  }

  candidates.sort((a, b) => b.playCount - a.playCount)
  return candidates
}

export function computeOnThisDay(
  sessions: PlaybackSession[],
  trackIndex: Map<number, Track>,
): EnrichedSession[] {
  const now = new Date()
  const month = now.getMonth()
  const day = now.getDate()
  const year = now.getFullYear()

  return sessions
    .filter((s) => {
      const d = new Date(s.startedAt)
      return d.getMonth() === month && d.getDate() === day && d.getFullYear() !== year
    })
    .map((session) => ({
      session,
      track: trackIndex.get(session.mediaItemId),
    }))
    .sort((a, b) => new Date(b.session.startedAt).getTime() - new Date(a.session.startedAt).getTime())
}

export function computeDailyActivity(sessions: PlaybackSession[]): DayActivity[] {
  const days = 364
  const now = new Date()
  now.setHours(0, 0, 0, 0)

  const start = new Date(now)
  start.setDate(start.getDate() - days + 1)

  const dayMap = new Map<string, number>()

  for (const session of sessions) {
    const d = new Date(session.startedAt)
    d.setHours(0, 0, 0, 0)
    if (d < start) continue

    const key = d.toISOString().slice(0, 10)
    dayMap.set(key, (dayMap.get(key) ?? 0) + session.durationMs / 60000)
  }

  const result: DayActivity[] = []
  const cursor = new Date(start)
  for (let i = 0; i < days; i++) {
    const key = cursor.toISOString().slice(0, 10)
    result.push({ date: key, durationMinutes: Math.round(dayMap.get(key) ?? 0) })
    cursor.setDate(cursor.getDate() + 1)
  }

  return result
}

export function computeListeningStats(sessions: PlaybackSession[]): ListeningStats {
  const now = new Date()
  const todayStart = new Date(now)
  todayStart.setHours(0, 0, 0, 0)
  const weekStart = new Date(todayStart)
  weekStart.setDate(weekStart.getDate() - weekStart.getDay())
  const monthStart = new Date(now.getFullYear(), now.getMonth(), 1)

  let allTimeMs = 0
  let thisMonthMs = 0
  let thisWeekMs = 0
  let todayMs = 0
  const activeDaySet = new Set<string>()

  for (const session of sessions) {
    const startTime = new Date(session.startedAt).getTime()
    allTimeMs += session.durationMs

    const dayKey = new Date(session.startedAt).toISOString().slice(0, 10)
    activeDaySet.add(dayKey)

    if (startTime >= todayStart.getTime()) todayMs += session.durationMs
    if (startTime >= weekStart.getTime()) thisWeekMs += session.durationMs
    if (startTime >= monthStart.getTime()) thisMonthMs += session.durationMs
  }

  const activeDays = activeDaySet.size
  const avgDailyMs = activeDays > 0 ? Math.round(allTimeMs / activeDays) : 0

  return {
    allTimeMs,
    thisMonthMs,
    thisWeekMs,
    todayMs,
    avgDailyMs,
    totalSessions: sessions.length,
    activeDays,
  }
}

export function computeTopTracks(
  playRecords: Map<number, TrackPlayRecord>,
  trackIndex: Map<number, Track>,
  limit = 10,
): TopItem[] {
  const items: TopItem[] = []

  for (const [mediaItemId, record] of playRecords) {
    const track = trackIndex.get(mediaItemId)
    if (!track) continue

    items.push({
      name: `${track.title} — ${track.artist}`,
      count: record.playCount,
      durationMs: record.totalDurationMs,
      id: track.id,
    })
  }

  items.sort((a, b) => b.count - a.count || b.durationMs - a.durationMs)
  return items.slice(0, limit)
}

export function computeTopArtists(
  playRecords: Map<number, TrackPlayRecord>,
  trackIndex: Map<number, Track>,
  limit = 5,
): TopItem[] {
  const artistMap = new Map<string, { count: number; durationMs: number }>()

  for (const [mediaItemId, record] of playRecords) {
    const track = trackIndex.get(mediaItemId)
    if (!track) continue

    const existing = artistMap.get(track.artist)
    if (existing) {
      existing.count += record.playCount
      existing.durationMs += record.totalDurationMs
    } else {
      artistMap.set(track.artist, {
        count: record.playCount,
        durationMs: record.totalDurationMs,
      })
    }
  }

  const items: TopItem[] = Array.from(artistMap.entries()).map(([name, data]) => ({
    name,
    count: data.count,
    durationMs: data.durationMs,
  }))

  items.sort((a, b) => b.count - a.count || b.durationMs - a.durationMs)
  return items.slice(0, limit)
}

export function computeTopAlbums(
  playRecords: Map<number, TrackPlayRecord>,
  trackIndex: Map<number, Track>,
  limit = 5,
): TopItem[] {
  const albumMap = new Map<string, { count: number; durationMs: number; id?: number }>()

  for (const [mediaItemId, record] of playRecords) {
    const track = trackIndex.get(mediaItemId)
    if (!track) continue

    const key = `${track.album} — ${track.artist}`
    const existing = albumMap.get(key)
    if (existing) {
      existing.count += record.playCount
      existing.durationMs += record.totalDurationMs
    } else {
      albumMap.set(key, {
        count: record.playCount,
        durationMs: record.totalDurationMs,
        id: track.id,
      })
    }
  }

  const items: TopItem[] = Array.from(albumMap.entries()).map(([name, data]) => ({
    name,
    count: data.count,
    durationMs: data.durationMs,
    id: data.id,
  }))

  items.sort((a, b) => b.count - a.count || b.durationMs - a.durationMs)
  return items.slice(0, limit)
}

export function computeYearInReview(
  sessions: PlaybackSession[],
  trackIndex: Map<number, Track>,
  year?: number,
): YearInReview {
  const targetYear = year ?? (new Date().getMonth() === 0 ? new Date().getFullYear() - 1 : new Date().getFullYear())

  const yearSessions = sessions.filter((s) => new Date(s.startedAt).getFullYear() === targetYear)

  let totalMs = 0
  const activeDaySet = new Set<string>()
  const monthBuckets = new Map<number, { durationMs: number; sessions: number }>()

  for (const session of yearSessions) {
    totalMs += session.durationMs
    const d = new Date(session.startedAt)
    activeDaySet.add(d.toISOString().slice(0, 10))

    const m = d.getMonth()
    const bucket = monthBuckets.get(m)
    if (bucket) {
      bucket.durationMs += session.durationMs
      bucket.sessions++
    } else {
      monthBuckets.set(m, { durationMs: session.durationMs, sessions: 1 })
    }
  }

  const monthlyBreakdown = Array.from({ length: 12 }, (_, i) => ({
    month: i,
    durationMs: monthBuckets.get(i)?.durationMs ?? 0,
    sessions: monthBuckets.get(i)?.sessions ?? 0,
  }))

  let mostActiveMonth: YearInReview['mostActiveMonth'] = null
  for (const mb of monthlyBreakdown) {
    if (mb.durationMs > 0 && (!mostActiveMonth || mb.durationMs > mostActiveMonth.durationMs)) {
      mostActiveMonth = { month: mb.month, durationMs: mb.durationMs }
    }
  }

  const playRecords = buildPlayRecords(yearSessions)

  return {
    year: targetYear,
    totalMs,
    totalSessions: yearSessions.length,
    activeDays: activeDaySet.size,
    topTracks: computeTopTracks(playRecords, trackIndex, 5),
    topArtists: computeTopArtists(playRecords, trackIndex, 3),
    topAlbums: computeTopAlbums(playRecords, trackIndex, 3),
    mostActiveMonth,
    monthlyBreakdown,
  }
}

// --- Listening DNA ---

export interface ListeningDna {
  artistDiversity: {
    uniqueArtists: number
    totalPlays: number
    ratio: number
    entropy: number
    label: string
  }
  albumDepth: {
    avgTracksPerAlbum: number
    albumsStarted: number
    albumsCompleted: number
    completionRate: number
    label: string
  }
  sessionPatterns: {
    avgSessionMinutes: number
    peakHour: number
    peakDay: number
    sessionsPerWeek: number
    label: string
  }
  formatPreferences: {
    losslessPct: number
    dominantFormat: string
    label: string
  }
  listeningVelocity: {
    tracksPerWeek: number[]
    trend: 'accelerating' | 'steady' | 'decelerating'
    label: string
  }
}

const LOSSLESS_FORMATS = new Set(['flac', 'alac', 'wav', 'aiff', 'dsd', 'dsf'])

function shannonEntropy(counts: number[]): number {
  const total = counts.reduce((a, b) => a + b, 0)
  if (total === 0) return 0
  let h = 0
  for (const c of counts) {
    if (c === 0) continue
    const p = c / total
    h -= p * Math.log2(p)
  }
  return h
}

function isoWeek(d: Date): string {
  const date = new Date(d)
  date.setHours(0, 0, 0, 0)
  date.setDate(date.getDate() + 3 - ((date.getDay() + 6) % 7))
  const week1 = new Date(date.getFullYear(), 0, 4)
  const weekNum = 1 + Math.round(((date.getTime() - week1.getTime()) / 86400000 - 3 + ((week1.getDay() + 6) % 7)) / 7)
  return `${date.getFullYear()}-W${String(weekNum).padStart(2, '0')}`
}

export function computeListeningDna(
  sessions: PlaybackSession[],
  playRecords: Map<number, TrackPlayRecord>,
  trackIndex: Map<number, Track>,
): ListeningDna {
  // Artist diversity
  const artistCounts = new Map<string, number>()
  let totalPlays = 0
  for (const [id, record] of playRecords) {
    const track = trackIndex.get(id)
    if (!track) continue
    artistCounts.set(track.artist, (artistCounts.get(track.artist) ?? 0) + record.playCount)
    totalPlays += record.playCount
  }
  const uniqueArtists = artistCounts.size
  const artistEntropy = shannonEntropy(Array.from(artistCounts.values()))
  const diversityRatio = totalPlays > 0 ? uniqueArtists / totalPlays : 0
  const diversityLabel = artistEntropy > 5 ? 'Explorer' : artistEntropy > 3.5 ? 'Curious' : artistEntropy > 2 ? 'Focused' : 'Loyalist'

  // Album depth
  const albumTrackSets = new Map<string, Set<number>>()
  const albumTotalCounts = new Map<string, number>()
  for (const track of trackIndex.values()) {
    if (!track.album) continue
    const key = `${track.album}|||${track.artist}`
    if (!albumTotalCounts.has(key)) albumTotalCounts.set(key, 0)
    albumTotalCounts.set(key, (albumTotalCounts.get(key) ?? 0) + 1)
  }
  for (const [id] of playRecords) {
    const track = trackIndex.get(id)
    if (!track?.album) continue
    const key = `${track.album}|||${track.artist}`
    if (!albumTrackSets.has(key)) albumTrackSets.set(key, new Set())
    albumTrackSets.get(key)!.add(id)
  }
  const albumsStarted = albumTrackSets.size
  let albumsCompleted = 0
  let totalTracksPlayed = 0
  for (const [key, played] of albumTrackSets) {
    totalTracksPlayed += played.size
    const total = albumTotalCounts.get(key) ?? 0
    if (total > 0 && played.size >= total) albumsCompleted++
  }
  const avgTracksPerAlbum = albumsStarted > 0 ? totalTracksPlayed / albumsStarted : 0
  const completionRate = albumsStarted > 0 ? albumsCompleted / albumsStarted : 0
  const depthLabel = completionRate > 0.6 ? 'Completionist' : completionRate > 0.3 ? 'Deep Diver' : avgTracksPerAlbum > 3 ? 'Sampler' : 'Cherry Picker'

  // Session patterns
  const hourBuckets = new Array(24).fill(0)
  const dayBuckets = new Array(7).fill(0)
  let totalSessionMs = 0
  for (const s of sessions) {
    const d = new Date(s.startedAt)
    hourBuckets[d.getHours()]++
    dayBuckets[d.getDay()]++
    totalSessionMs += s.durationMs
  }
  const peakHour = hourBuckets.indexOf(Math.max(...hourBuckets))
  const peakDay = dayBuckets.indexOf(Math.max(...dayBuckets))
  const avgSessionMin = sessions.length > 0 ? totalSessionMs / sessions.length / 60000 : 0

  const weekSet = new Set<string>()
  for (const s of sessions) weekSet.add(isoWeek(new Date(s.startedAt)))
  const weeksActive = weekSet.size || 1
  const sessionsPerWeek = sessions.length / weeksActive

  const patternLabel = avgSessionMin > 60 ? 'Marathon Listener' : avgSessionMin > 20 ? 'Session Listener' : avgSessionMin > 5 ? 'Quick Listener' : 'Grazer'

  // Format preferences
  const formatCounts = new Map<string, number>()
  let losslessCount = 0
  for (const [id, record] of playRecords) {
    const track = trackIndex.get(id)
    if (!track) continue
    const fmt = track.format?.toLowerCase() ?? 'unknown'
    formatCounts.set(fmt, (formatCounts.get(fmt) ?? 0) + record.playCount)
    if (LOSSLESS_FORMATS.has(fmt)) losslessCount += record.playCount
  }
  const losslessPct = totalPlays > 0 ? (losslessCount / totalPlays) * 100 : 0
  let dominantFormat = 'unknown'
  let maxFmt = 0
  for (const [fmt, cnt] of formatCounts) {
    if (cnt > maxFmt) { maxFmt = cnt; dominantFormat = fmt }
  }
  const formatLabel = losslessPct > 80 ? 'Audiophile' : losslessPct > 40 ? 'Quality Seeker' : losslessPct > 10 ? 'Pragmatist' : 'Casual'

  // Listening velocity (last 12 weeks)
  const now = new Date()
  const weeklyTracks = new Map<string, Set<number>>()
  const last12Weeks: string[] = []
  for (let i = 11; i >= 0; i--) {
    const d = new Date(now)
    d.setDate(d.getDate() - i * 7)
    last12Weeks.push(isoWeek(d))
  }
  for (const s of sessions) {
    const wk = isoWeek(new Date(s.startedAt))
    if (!weeklyTracks.has(wk)) weeklyTracks.set(wk, new Set())
    weeklyTracks.get(wk)!.add(s.mediaItemId)
  }
  const tracksPerWeek = last12Weeks.map((wk) => weeklyTracks.get(wk)?.size ?? 0)
  const first4Avg = tracksPerWeek.slice(0, 4).reduce((a, b) => a + b, 0) / 4
  const last4Avg = tracksPerWeek.slice(-4).reduce((a, b) => a + b, 0) / 4
  const trendDelta = last4Avg - first4Avg
  const trend = trendDelta > 2 ? 'accelerating' : trendDelta < -2 ? 'decelerating' : 'steady'
  const velocityLabel = trend === 'accelerating' ? 'Ramping Up' : trend === 'decelerating' ? 'Winding Down' : 'Steady State'

  return {
    artistDiversity: { uniqueArtists, totalPlays, ratio: diversityRatio, entropy: artistEntropy, label: diversityLabel },
    albumDepth: { avgTracksPerAlbum, albumsStarted, albumsCompleted, completionRate, label: depthLabel },
    sessionPatterns: { avgSessionMinutes: avgSessionMin, peakHour, peakDay, sessionsPerWeek, label: patternLabel },
    formatPreferences: { losslessPct, dominantFormat, label: formatLabel },
    listeningVelocity: { tracksPerWeek, trend, label: velocityLabel },
  }
}

export function computeNewForYou(
  playRecords: Map<number, TrackPlayRecord>,
  trackIndex: Map<number, Track>,
  tracks: Track[],
  limit = 20,
): NewForYouItem[] {
  const artistCounts = new Map<string, number>()
  for (const [mediaItemId, record] of playRecords) {
    const track = trackIndex.get(mediaItemId)
    if (!track) continue
    artistCounts.set(track.artist, (artistCounts.get(track.artist) ?? 0) + record.playCount)
  }

  const playedIds = new Set(playRecords.keys())

  const albumTracks = new Map<string, Track[]>()
  for (const track of tracks) {
    if (!track.album) continue
    const key = `${track.album}|||${track.artist}`
    const existing = albumTracks.get(key) ?? []
    existing.push(track)
    albumTracks.set(key, existing)
  }

  const items: NewForYouItem[] = []

  for (const track of tracks) {
    if (playedIds.has(track.id)) continue
    const artistCount = artistCounts.get(track.artist)
    if (!artistCount || artistCount < 2) continue

    const albumKey = `${track.album}|||${track.artist}`
    const albumTrackList = albumTracks.get(albumKey) ?? []
    const playedFromAlbum = albumTrackList.filter((t) => playedIds.has(t.id)).length

    if (playedFromAlbum > 0 && playedFromAlbum < albumTrackList.length) {
      items.push({
        track,
        reason: 'incomplete_album',
        artistPlayCount: artistCount,
        albumCompletionPct: Math.round((playedFromAlbum / albumTrackList.length) * 100),
      })
    } else {
      items.push({
        track,
        reason: 'unheard_from_favorite_artist',
        artistPlayCount: artistCount,
      })
    }
  }

  items.sort((a, b) => {
    if (a.reason === 'incomplete_album' && b.reason !== 'incomplete_album') return -1
    if (b.reason === 'incomplete_album' && a.reason !== 'incomplete_album') return 1
    if (a.reason === 'incomplete_album' && b.reason === 'incomplete_album') {
      return (b.albumCompletionPct ?? 0) - (a.albumCompletionPct ?? 0)
    }
    return b.artistPlayCount - a.artistPlayCount
  })

  return items.slice(0, limit)
}
