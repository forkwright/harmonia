// Podcast browsing and episode playback
import { useEffect, useState } from 'react'
import { usePodcastStore } from '../stores/podcastStore'
import { usePlayerStore } from '../stores/playerStore'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import { Input } from '../components/Input'
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
        className={`w-full text-left bg-bronze-800/50 rounded-lg border transition-colors overflow-hidden ${
          selected
            ? 'border-bronze-400 bg-bronze-800'
            : 'border-bronze-700/30 hover:bg-bronze-800'
        }`}
      >
        <div className="flex gap-4 p-4">
          <div className="w-20 h-20 flex-shrink-0 bg-bronze-700 rounded overflow-hidden">
            {show.imageUrl ? (
              <img
                src={show.imageUrl}
                alt={show.title}
                className="w-full h-full object-cover"
                onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
              />
            ) : (
              <div className="w-full h-full flex items-center justify-center text-bronze-400">
                <svg className="w-8 h-8" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217z" clipRule="evenodd"/>
                </svg>
              </div>
            )}
          </div>
          <div className="min-w-0 flex-1">
            <h3 className="text-base font-semibold text-bronze-100 truncate">{show.title}</h3>
            {show.author && (
              <p className="text-sm text-bronze-400 mt-0.5 truncate">{show.author}</p>
            )}
            <div className="flex items-center gap-3 mt-1 text-xs text-bronze-500">
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
        className="absolute top-2 right-2 w-6 h-6 rounded-full bg-bronze-900/80 text-bronze-500 hover:text-red-400 hover:bg-bronze-900 opacity-0 group-hover:opacity-100 transition-all flex items-center justify-center text-sm"
        title={`Unsubscribe from ${show.title}`}
        aria-label={`Unsubscribe from ${show.title}`}
      >
        &times;
      </button>
    </div>
  )
}

function EpisodeRow({ episode, show, onPlay }: { episode: PodcastEpisode; show: PodcastShow; onPlay: () => void }) {
  return (
    <div className="flex items-start gap-3 p-4 bg-bronze-800/50 rounded-lg border border-bronze-700/30 hover:bg-bronze-800 transition-colors">
      {(episode.imageUrl ?? show.imageUrl) && (
        <div className="w-12 h-12 flex-shrink-0 bg-bronze-700 rounded overflow-hidden">
          <img
            src={episode.imageUrl ?? show.imageUrl}
            alt={episode.title}
            className="w-full h-full object-cover"
            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
          />
        </div>
      )}
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-bronze-100 line-clamp-2">{episode.title}</p>
        <div className="flex items-center gap-3 mt-1 text-xs text-bronze-500">
          {episode.publishDate && <span>{formatDate(episode.publishDate)}</span>}
          {episode.duration != null && episode.duration > 0 && (
            <span>{formatDuration(episode.duration)}</span>
          )}
          {episode.episodeNumber != null && (
            <span>Ep. {episode.episodeNumber}</span>
          )}
          {episode.explicit && (
            <span className="px-1.5 py-0.5 bg-bronze-700 rounded text-bronze-300 text-xs">E</span>
          )}
        </div>
      </div>
      <button
        onClick={onPlay}
        disabled={!episode.enclosureUrl}
        className="flex-shrink-0 w-9 h-9 rounded-full bg-bronze-700 hover:bg-bronze-600 disabled:opacity-40 disabled:cursor-not-allowed flex items-center justify-center transition-colors"
        title={episode.enclosureUrl ? `Play ${episode.title}` : 'No stream available'}
        aria-label={`Play ${episode.title}`}
      >
        <svg className="w-4 h-4 text-bronze-100 ml-0.5" fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
        </svg>
      </button>
    </div>
  )
}

export function PodcastsPage() {
  const {
    shows, selectedShow, episodes, isLoading, error,
    fetchShows, selectShow, clearSelection,
    playEpisode, subscribePodcast, unsubscribePodcast,
  } = usePodcastStore()
  const { setCurrentTrack, setIsPlaying } = usePlayerStore()

  const [showAddForm, setShowAddForm] = useState(false)
  const [feedUrl, setFeedUrl] = useState('')

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
        <h1 className="text-2xl font-bold text-bronze-100">Podcasts</h1>
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
        <div className="text-center py-12 text-bronze-400">Loading...</div>
      )}

      {!isLoading && !error && shows.length === 0 && !showAddForm && (
        <div className="text-center py-12 text-bronze-400">No podcasts found.</div>
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
              <h2 className="text-xl font-bold text-bronze-100">{selectedShow.title}</h2>
              {selectedShow.description && (
                <p className="text-sm text-bronze-400 mt-1 line-clamp-3">{selectedShow.description}</p>
              )}
            </div>

            {isLoading ? (
              <div className="text-center py-8 text-bronze-400">Loading episodes...</div>
            ) : episodes.length === 0 ? (
              <div className="text-center py-8 text-bronze-400">No episodes found.</div>
            ) : (
              <div className="space-y-2">
                {episodes.map((ep) => (
                  <EpisodeRow
                    key={ep.id}
                    episode={ep}
                    show={selectedShow}
                    onPlay={() => handlePlayEpisode(ep)}
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
