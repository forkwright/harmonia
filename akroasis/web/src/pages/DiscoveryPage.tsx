// Discovery dashboard — orchestrator for all discovery sections
import { useEffect, useMemo } from 'react'
import { useDiscoveryStore } from '../stores/discoveryStore'
import { ContinueListening } from '../components/ContinueListening'
import { ListeningStatsSection } from '../components/discovery/ListeningStatsSection'
import { OnThisDaySection } from '../components/discovery/OnThisDaySection'
import { RediscoverSection } from '../components/discovery/RediscoverSection'
import { TopListsSection } from '../components/discovery/TopListsSection'
import { ListeningDnaSection } from '../components/discovery/ListeningDnaSection'
import { YearInReviewSection } from '../components/discovery/YearInReviewSection'
import { RecentlyAddedSection } from '../components/discovery/RecentlyAddedSection'
import { NewForYouSection } from '../components/discovery/NewForYouSection'
import {
  buildTrackIndex,
  buildPlayRecords,
  computeListeningStats,
  computeDailyActivity,
  computeOnThisDay,
  computeRediscoverCandidates,
  computeTopTracks,
  computeTopArtists,
  computeTopAlbums,
  computeYearInReview,
  computeNewForYou,
  computeListeningDna,
} from '../utils/discoveryStats'

export function DiscoveryPage() {
  const { sessions, recentHistory, tracks, isLoading, error, fetchAll } =
    useDiscoveryStore()

  useEffect(() => {
    void fetchAll()
  }, [fetchAll])

  const trackIndex = useMemo(() => buildTrackIndex(tracks), [tracks])
  const playRecords = useMemo(() => buildPlayRecords(sessions), [sessions])

  const stats = useMemo(() => computeListeningStats(sessions), [sessions])
  const dailyActivity = useMemo(() => computeDailyActivity(sessions), [sessions])
  const onThisDay = useMemo(() => computeOnThisDay(sessions, trackIndex), [sessions, trackIndex])
  const rediscover = useMemo(() => computeRediscoverCandidates(playRecords, trackIndex), [playRecords, trackIndex])
  const topTracks = useMemo(() => computeTopTracks(playRecords, trackIndex), [playRecords, trackIndex])
  const topArtists = useMemo(() => computeTopArtists(playRecords, trackIndex), [playRecords, trackIndex])
  const topAlbums = useMemo(() => computeTopAlbums(playRecords, trackIndex), [playRecords, trackIndex])
  const yearInReview = useMemo(() => computeYearInReview(sessions, trackIndex), [sessions, trackIndex])
  const newForYou = useMemo(() => computeNewForYou(playRecords, trackIndex, tracks), [playRecords, trackIndex, tracks])
  const listeningDna = useMemo(() => computeListeningDna(sessions, playRecords, trackIndex), [sessions, playRecords, trackIndex])

  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-bronze-900">Discovery</h1>
        <p className="text-bronze-600 mt-1 text-sm">
          {new Date().toLocaleDateString(undefined, {
            weekday: 'long',
            year: 'numeric',
            month: 'long',
            day: 'numeric',
          })}
        </p>
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded mb-6 text-sm">
          {error}
        </div>
      )}

      {isLoading && sessions.length === 0 && tracks.length === 0 ? (
        <div className="text-center text-bronze-600 py-12">Loading...</div>
      ) : (
        <div className="space-y-6">
          <ContinueListening />

          <NewForYouSection items={newForYou} />

          <ListeningStatsSection stats={stats} dailyActivity={dailyActivity} />

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <OnThisDaySection sessions={onThisDay} />
            <RediscoverSection candidates={rediscover} />
          </div>

          <TopListsSection
            topTracks={topTracks}
            topArtists={topArtists}
            topAlbums={topAlbums}
          />

          <ListeningDnaSection dna={listeningDna} />

          <YearInReviewSection review={yearInReview} />

          <RecentlyAddedSection entries={recentHistory} />
        </div>
      )}
    </div>
  )
}
