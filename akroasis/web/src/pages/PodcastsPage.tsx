// Podcast browsing and episode playback
import { useEffect, useState, useMemo } from 'react'
import { usePodcastStore } from '../stores/podcastStore'
import type { EpisodeFilter } from '../stores/podcastStore'
import { usePlayerStore } from '../stores/playerStore'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import { Input } from '../components/Input'
import { authenticateUrl } from '../api/client'
import type { PodcastEpisode, PodcastShow, Track } from '../types'

function formatDuration(seconds?: number): string {
  if (!seconds) return ''
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = seconds % 60
  if (h > 0) return `${h}h ${m}m`
  if (m > 0) return `${m}m ${s}s`
  return `${s}s`
}

function formatDate(iso?: string): string {
  if (!iso) return ''
  return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' })
}

function episodeToTrack(episode: PodcastEpisode, show: PodcastShow): Track {
  return {
    id: episode.id,
    title: episode.title,
    artist: show.author ?? show.title,
    album: show.title,
    duration: episode.duration ?? 0,
    fileSize: 0,
    format: episode.enclosureType?.split('/')[1] ?? 'mp3',
    bitrate: 0,
    sampleRate: 44100,
    bitDepth: 16,
    channels: 2,
    coverArtUrl: episode.imageUrl ?? show.imageUrl,
  }
}

interface PodcastTrack extends Track {
  streamUrl?: string
}

function episodeToPodcastTrack(episode: PodcastEpisode, show: PodcastShow): PodcastTrack {
  return {
    ...episodeToTrack(episode, show),
    streamUrl: episode.enclosureUrl,
  }
}

function ShowCard({
  show,
  selected,
  onClick,
  onUnsubscribe,
}: {
  show: PodcastShow
  selected: boolean
  onClick: () => void
  onUnsubscribe: () => void
}) {
  return (
    <div className="relative group">
      <button
        onClick={onClick}
        className={`w-full text-left bg-accent-subtle rounded-lg border transition-colors overflow-hidden ${
          selected
            ? 'border-accent bg-surface-sunken'
            : 'border-theme-subtle hover:bg-surface-sunken'
        }`}
      >
        <div className="flex gap-4 p-4">
          <div className="w-20 h-20 flex-shrink-0 bg-surface-sunken rounded overflow-hidden">
            {show.imageUrl ? (
              <img
                src={authenticateUrl(show.imageUrl)}
                alt={show.title}
                className="w-full h-full object-cover"
                onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
              />
            ) : (
              <div className="w-full h-full flex items-center justify-center text-theme-tertiary">
                <svg className="w-8 h-8" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217z" clipRule="evenodd"/>
                </svg>
              </div>
            )}
          </div>
          <div className="min-w-0 flex-1">
            <h3 className="text-base font-semibold text-theme-primary truncate">{show.title}</h3>
            {show.author && (
              <p className="text-sm text-theme-tertiary mt-0.5 truncate">{show.author}</p>
            )}
            <div className="flex items-center gap-3 mt-1 text-xs text-theme-tertiary">
              {show.episodeCount != null && (
                <span>{show.episodeCount} episodes</span>
              )}
              {show.latestEpisodeDate && (
                <span>Latest: {formatDate(show.latestEpisodeDate)}</span>
              )}
            </div>
          </div>
        </div>
      </button>
      <button
        onClick={(e) => { e.stopPropagation(); onUnsubscribe() }}
        className="absolute top-2 right-2 w-6 h-6 rounded-full bg-surface-raised text-theme-tertiary hover:text-red-400 hover:bg-surface-raised opacity-0 group-hover:opacity-100 transition-all flex items-center justify-center text-sm"
        title={`Unsubscribe from ${show.title}`}
        aria-label={`Unsubscribe from ${show.title}`}
      >
        &times;
      </button>
    </div>
  )
}

function EpisodeRow({ episode, show, played, onPlay, onTogglePlayed }: {
  episode: PodcastEpisode; show: PodcastShow; played: boolean;
  onPlay: () => void; onTogglePlayed: () => void;
}) {
  return (
    <div className={`flex items-start gap-3 p-4 bg-accent-subtle rounded-lg border border-theme-subtle hover:bg-surface-sunken transition-colors ${played ? 'opacity-60' : ''}`}>
      {(episode.imageUrl ?? show.imageUrl) && (
        <div className="w-12 h-12 flex-shrink-0 bg-surface-sunken rounded overflow-hidden">
          <img
            src={authenticateUrl(episode.imageUrl ?? show.imageUrl)}
            alt={episode.title}
            className="w-full h-full object-cover"
            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
          />
        </div>
      )}
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          {played && (
            <svg className="w-3.5 h-3.5 text-theme-tertiary flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd"/>
            </svg>
          )}
          <p className="text-sm font-medium text-theme-primary line-clamp-2">{episode.title}</p>
        </div>
        <div className="flex items-center gap-3 mt-1 text-xs text-theme-tertiary">
          {episode.publishDate && <span>{formatDate(episode.publishDate)}</span>}
          {episode.duration != null && episode.duration > 0 && (
            <span>{formatDuration(episode.duration)}</span>
          )}
          {episode.episodeNumber != null && (
            <span>Ep. {episode.episodeNumber}</span>
          )}
          {episode.explicit && (
            <span className="px-1.5 py-0.5 bg-surface-sunken rounded text-theme-secondary text-xs">E</span>
          )}
        </div>
      </div>
      <button
        onClick={onTogglePlayed}
        className="flex-shrink-0 w-7 h-7 rounded-full text-theme-muted hover:text-theme-secondary flex items-center justify-center transition-colors"
        title={played ? 'Mark as unplayed' : 'Mark as played'}
        aria-label={played ? `Mark ${episode.title} as unplayed` : `Mark ${episode.title} as played`}
      >
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
          {played ? (
            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
          ) : (
            <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
          )}
        </svg>
      </button>
      <button
        onClick={onPlay}
        disabled={!episode.enclosureUrl}
        className="flex-shrink-0 w-9 h-9 rounded-full bg-surface-sunken hover:bg-accent disabled:opacity-40 disabled:cursor-not-allowed flex items-center justify-center transition-colors"
        title={episode.enclosureUrl ? `Play ${episode.title}` : 'No stream available'}
        aria-label={`Play ${episode.title}`}
      >
        <svg className="w-4 h-4 text-theme-primary ml-0.5" fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
        </svg>
      </button>
    </div>
  )
}

export function PodcastsPage() {
  const {
    shows, selectedShow, episodes, isLoading, error,
    playedEpisodes, episodeFilter, autoMarkPlayed,
    fetchShows, selectShow, clearSelection,
    playEpisode, subscribePodcast, unsubscribePodcast,
    togglePlayed, setEpisodeFilter, setAutoMarkPlayed,
  } = usePodcastStore()
  const { setCurrentTrack, setIsPlaying } = usePlayerStore()

  const [showAddForm, setShowAddForm] = useState(false)
  const [feedUrl, setFeedUrl] = useState('')

  const filteredEpisodes = useMemo(() => {
    if (episodeFilter === 'all') return episodes
    return episodes.filter((ep) => {
      const isPlayed = !!playedEpisodes[ep.id]?.played
      return episodeFilter === 'played' ? isPlayed : !isPlayed
    })
  }, [episodes, episodeFilter, playedEpisodes])

  useEffect(() => {
    fetchShows()
  }, [fetchShows])

  function handleSelectShow(show: PodcastShow) {
    if (selectedShow?.id === show.id) {
      clearSelection()
    } else {
      selectShow(show.id)
    }
  }

  function handlePlayEpisode(episode: PodcastEpisode) {
    if (!selectedShow || !episode.enclosureUrl) return
    const track = episodeToPodcastTrack(episode, selectedShow)
    playEpisode(episode)
    setCurrentTrack(track)
    setIsPlaying(true)
  }

  async function handleSubscribe() {
    const url = feedUrl.trim()
    if (!url) return
    await subscribePodcast(url)
    setFeedUrl('')
    setShowAddForm(false)
  }

  return (
    <div className="max-w-7xl mx-auto px-4 py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-serif font-semibold" style={{ color: 'rgb(var(--text-primary))' }}>Podcasts</h1>
        <div className="flex items-center gap-2">
          {selectedShow && (
            <Button variant="ghost" onClick={clearSelection}>
              Clear selection
            </Button>
          )}
          <Button
            variant="secondary"
            onClick={() => setShowAddForm(!showAddForm)}
          >
            {showAddForm ? 'Cancel' : '+ Add Podcast'}
          </Button>
        </div>
      </div>

      {showAddForm && (
        <Card>
          <div className="flex items-end gap-3 mb-6">
            <div className="flex-1">
              <Input
                label="Podcast Feed URL"
                value={feedUrl}
                onChange={(e) => setFeedUrl(e.target.value)}
                placeholder="https://example.com/feed.xml"
                onKeyDown={(e) => { if (e.key === 'Enter') void handleSubscribe() }}
              />
            </div>
            <Button onClick={() => void handleSubscribe()} disabled={!feedUrl.trim() || isLoading}>
              Subscribe
            </Button>
          </div>
        </Card>
      )}

      {error && (
        <Card>
          <p className="text-red-400">{error}</p>
        </Card>
      )}

      {isLoading && !shows.length && (
        <div className="text-center py-12 text-theme-tertiary">Loading...</div>
      )}

      {!isLoading && !error && shows.length === 0 && !showAddForm && (
        <div className="text-center py-12 text-theme-tertiary">No podcasts found.</div>
      )}

      <div className="flex gap-6">
        <div className={`${selectedShow ? 'w-1/2' : 'w-full'} transition-all`}>
          <div className={`grid gap-4 ${selectedShow ? 'grid-cols-1' : 'grid-cols-1 md:grid-cols-2 lg:grid-cols-3'}`}>
            {shows.map((show) => (
              <ShowCard
                key={show.id}
                show={show}
                selected={selectedShow?.id === show.id}
                onClick={() => handleSelectShow(show)}
                onUnsubscribe={() => void unsubscribePodcast(show.id)}
              />
            ))}
          </div>
        </div>

        {selectedShow && (
          <div className="w-1/2">
            <div className="mb-4">
              <h2 className="text-xl font-bold text-theme-primary">{selectedShow.title}</h2>
              {selectedShow.description && (
                <p className="text-sm text-theme-tertiary mt-1 line-clamp-3">{selectedShow.description}</p>
              )}
            </div>

            <div className="flex items-center justify-between mb-3">
              <div className="flex gap-1.5">
                {(['all', 'unplayed', 'played'] as EpisodeFilter[]).map((f) => (
                  <button
                    key={f}
                    onClick={() => setEpisodeFilter(f)}
                    className={`px-2.5 py-0.5 rounded-full text-xs transition-colors capitalize ${
                      episodeFilter === f
                        ? 'bg-accent text-white'
                        : 'bg-surface-raised border border-theme-default text-theme-tertiary hover:border-accent hover:text-theme-primary'
                    }`}
                  >
                    {f}
                  </button>
                ))}
              </div>
              <label className="flex items-center gap-1.5 text-xs text-theme-tertiary cursor-pointer">
                <input
                  type="checkbox"
                  checked={autoMarkPlayed}
                  onChange={(e) => setAutoMarkPlayed(e.target.checked)}
                  className="rounded border-theme-strong bg-surface-raised text-theme-tertiary focus:ring-accent"
                />
                Auto-mark played
              </label>
            </div>

            {isLoading ? (
              <div className="text-center py-8 text-theme-tertiary">Loading episodes...</div>
            ) : filteredEpisodes.length === 0 ? (
              <div className="text-center py-8 text-theme-tertiary">
                {episodes.length === 0 ? 'No episodes found.' : `No ${episodeFilter} episodes.`}
              </div>
            ) : (
              <div className="space-y-2">
                {filteredEpisodes.map((ep) => (
                  <EpisodeRow
                    key={ep.id}
                    episode={ep}
                    show={selectedShow}
                    played={!!playedEpisodes[ep.id]?.played}
                    onPlay={() => handlePlayEpisode(ep)}
                    onTogglePlayed={() => togglePlayed(ep.id)}
                  />
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
