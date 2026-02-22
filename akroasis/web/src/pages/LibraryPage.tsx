import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { apiClient } from '../api/client'
import { isLastfmConfigured } from '../api/lastfm'
import type { Artist, Album, Track } from '../types'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import { usePlayerStore } from '../stores/playerStore'
import { useRadioStore } from '../stores/radioStore'
import { HeartButton } from '../components/HeartButton'
import { useArtworkViewer } from '../stores/artworkViewerStore'

type View = 'artists' | 'albums' | 'tracks'

function Skeleton({ className = '' }: { className?: string }) {
  return (
    <div className={`animate-pulse bg-bronze-800/50 rounded ${className}`} />
  )
}

function ArtistCardSkeleton() {
  return (
    <div className="bg-bronze-900 border border-bronze-800/50 rounded-xl p-5">
      <Skeleton className="h-6 w-3/4 mb-3" />
      <Skeleton className="h-4 w-1/2" />
    </div>
  )
}

function AlbumCardSkeleton() {
  return (
    <div className="bg-bronze-900 border border-bronze-800/50 rounded-xl overflow-hidden">
      <Skeleton className="w-full aspect-square rounded-none" />
      <div className="p-4">
        <Skeleton className="h-5 w-3/4 mb-2" />
        <Skeleton className="h-4 w-1/2 mb-2" />
        <Skeleton className="h-3 w-2/3" />
      </div>
    </div>
  )
}

function EmptyState({ icon, title, subtitle }: { icon: React.ReactNode; title: string; subtitle: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-20">
      <div className="w-20 h-20 rounded-2xl bg-bronze-900 flex items-center justify-center mb-4">
        {icon}
      </div>
      <p className="text-bronze-300 text-lg">{title}</p>
      <p className="text-bronze-600 text-sm mt-1">{subtitle}</p>
    </div>
  )
}

export function LibraryPage() {
  const [view, setView] = useState<View>('artists')
  const [artists, setArtists] = useState<Artist[]>([])
  const [albums, setAlbums] = useState<Album[]>([])
  const [tracks, setTracks] = useState<Track[]>([])
  const [selectedArtist, setSelectedArtist] = useState<Artist | null>(null)
  const [selectedAlbum, setSelectedAlbum] = useState<Album | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const navigate = useNavigate()
  const { setCurrentTrack, setIsPlaying } = usePlayerStore()
  const { startRadio } = useRadioStore()
  const radioEnabled = isLastfmConfigured()
  const openArtwork = useArtworkViewer((s) => s.open)

  useEffect(() => {
    loadArtists()
  }, [])

  async function loadArtists() {
    try {
      setLoading(true)
      setError(null)
      const data = await apiClient.getArtists()
      setArtists(data.items)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load artists')
    } finally {
      setLoading(false)
    }
  }

  async function loadAlbumsByArtist(artist: Artist) {
    try {
      setLoading(true)
      setError(null)
      setSelectedArtist(artist)
      const data = await apiClient.getAlbums(artist.id)
      setAlbums(data)
      setView('albums')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load albums')
    } finally {
      setLoading(false)
    }
  }

  async function loadTracksByAlbum(album: Album) {
    try {
      setLoading(true)
      setError(null)
      setSelectedAlbum(album)
      const data = await apiClient.getTracks(album.id)
      setTracks(data)
      setView('tracks')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load tracks')
    } finally {
      setLoading(false)
    }
  }

  function handleTrackSelect(track: Track) {
    setCurrentTrack(track)
    setIsPlaying(true)
    navigate('/player')
  }

  async function handleStartRadio(track: Track, e: React.MouseEvent) {
    e.stopPropagation()
    await startRadio(track)
    navigate('/queue')
  }

  function goBack() {
    if (view === 'tracks') {
      setView('albums')
    } else if (view === 'albums') {
      setView('artists')
      setSelectedArtist(null)
    }
  }

  return (
    <div className="max-w-7xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-8">
        <div className="flex items-center gap-4">
          {view !== 'artists' && (
            <Button onClick={goBack} variant="secondary" size="sm">
              <svg className="w-4 h-4 mr-1" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
              </svg>
              Back
            </Button>
          )}
          <div>
            <h1 className="text-3xl font-bold text-bronze-100">
              {view === 'artists' && 'Library'}
              {view === 'albums' && selectedArtist?.name}
              {view === 'tracks' && selectedAlbum?.title}
            </h1>
            {view === 'albums' && (
              <p className="text-bronze-500 text-sm mt-0.5">{albums.length} albums</p>
            )}
            {view === 'tracks' && selectedAlbum && (
              <p className="text-bronze-500 text-sm mt-0.5">
                {selectedArtist?.name} · {tracks.length} tracks
              </p>
            )}
          </div>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div className="bg-red-900/30 border border-red-700/50 text-red-300 px-4 py-3 rounded-lg mb-6 text-sm">
          {error}
        </div>
      )}

      {/* Loading skeletons */}
      {loading && view === 'artists' && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {Array.from({ length: 9 }).map((_, i) => <ArtistCardSkeleton key={i} />)}
        </div>
      )}

      {loading && view === 'albums' && (
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
          {Array.from({ length: 8 }).map((_, i) => <AlbumCardSkeleton key={i} />)}
        </div>
      )}

      {loading && view === 'tracks' && (
        <div className="space-y-2">
          {Array.from({ length: 10 }).map((_, i) => (
            <div key={i} className="bg-bronze-900 rounded-lg p-4">
              <Skeleton className="h-5 w-1/2 mb-2" />
              <Skeleton className="h-4 w-1/3" />
            </div>
          ))}
        </div>
      )}

      {/* Artists view */}
      {!loading && view === 'artists' && (
        artists.length === 0 ? (
          <EmptyState
            icon={<svg className="w-10 h-10 text-bronze-700" fill="currentColor" viewBox="0 0 20 20"><path d="M13 6a3 3 0 11-6 0 3 3 0 016 0zM18 8a2 2 0 11-4 0 2 2 0 014 0zM14 15a4 4 0 00-8 0v3h8v-3zM6 8a2 2 0 11-4 0 2 2 0 014 0zM16 18v-3a5.972 5.972 0 00-.75-2.906A3.005 3.005 0 0119 15v3h-3zM4.75 12.094A5.973 5.973 0 004 15v3H1v-3a3 3 0 013.75-2.906z"/></svg>}
            title="No artists found"
            subtitle="Add some music to your Mouseion library"
          />
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {artists.map(artist => (
              <Card
                key={artist.id}
                onClick={() => loadAlbumsByArtist(artist)}
                className="cursor-pointer hover:bg-bronze-800/80 hover:border-bronze-700 hover:scale-[1.01] transition-all duration-150"
              >
                <div className="p-1">
                  <h3 className="text-lg font-semibold text-bronze-100 mb-1">
                    {artist.name}
                  </h3>
                  <p className="text-bronze-500 text-sm">
                    {artist.albumCount} {artist.albumCount === 1 ? 'album' : 'albums'} · {artist.trackCount} tracks
                  </p>
                </div>
              </Card>
            ))}
          </div>
        )
      )}

      {/* Albums view */}
      {!loading && view === 'albums' && (
        albums.length === 0 ? (
          <EmptyState
            icon={<svg className="w-10 h-10 text-bronze-700" fill="currentColor" viewBox="0 0 20 20"><path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/></svg>}
            title="No albums found"
            subtitle={`No albums for ${selectedArtist?.name ?? 'this artist'}`}
          />
        ) : (
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
            {albums.map(album => (
              <div
                key={album.id}
                onClick={() => loadTracksByAlbum(album)}
                className="group cursor-pointer bg-bronze-900/50 rounded-xl overflow-hidden border border-bronze-800/30 hover:bg-bronze-800/50 hover:border-bronze-700/50 hover:scale-[1.02] transition-all duration-150"
                role="button"
                tabIndex={0}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); loadTracksByAlbum(album) }}}
              >
                <div className="w-full aspect-square bg-bronze-800 overflow-hidden">
                  {album.coverArtUrl ? (
                    <img
                      src={album.coverArtUrl}
                      alt={album.title}
                      className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                      onClick={(e) => { e.stopPropagation(); openArtwork(album.coverArtUrl!) }}
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center">
                      <svg className="w-12 h-12 text-bronze-700" fill="currentColor" viewBox="0 0 20 20">
                        <path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/>
                      </svg>
                    </div>
                  )}
                </div>
                <div className="p-3">
                  <h3 className="text-sm font-semibold text-bronze-100 truncate">
                    {album.title}
                  </h3>
                  <p className="text-bronze-500 text-xs mt-0.5 truncate">
                    {album.artist}
                  </p>
                  <p className="text-bronze-600 text-xs mt-1">
                    {album.year && `${album.year} · `}
                    {album.trackCount} tracks · {Math.floor(album.duration / 60)}min
                  </p>
                </div>
              </div>
            ))}
          </div>
        )
      )}

      {/* Tracks view */}
      {!loading && view === 'tracks' && (
        tracks.length === 0 ? (
          <EmptyState
            icon={<svg className="w-10 h-10 text-bronze-700" fill="currentColor" viewBox="0 0 20 20"><path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/></svg>}
            title="No tracks"
            subtitle="This album appears to be empty"
          />
        ) : (
          <div className="space-y-1">
            {tracks.map((track, index) => (
              <div
                key={track.id}
                onClick={() => handleTrackSelect(track)}
                className="flex items-center gap-4 px-4 py-3 rounded-lg cursor-pointer hover:bg-bronze-800/50 transition-colors group"
                role="button"
                tabIndex={0}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleTrackSelect(track) }}}
              >
                <span className="w-6 text-right text-sm text-bronze-600 tabular-nums group-hover:hidden">
                  {index + 1}
                </span>
                <svg className="w-6 h-6 text-bronze-400 hidden group-hover:block flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
                </svg>

                <div className="flex-1 min-w-0">
                  <h3 className="text-sm font-medium text-bronze-100 truncate">
                    {track.title}
                  </h3>
                  <p className="text-xs text-bronze-500 truncate">
                    {track.artist}
                  </p>
                </div>

                <span className="text-xs text-bronze-600 tabular-nums">
                  {track.format.toUpperCase()} · {(track.sampleRate / 1000).toFixed(1)}kHz/{track.bitDepth}bit
                </span>

                <HeartButton trackId={track.id} />

                {radioEnabled && (
                  <button
                    onClick={(e) => handleStartRadio(track, e)}
                    className="opacity-0 group-hover:opacity-100 text-bronze-500 hover:text-bronze-300 transition-all"
                    title="Start Radio"
                    aria-label={`Start radio from ${track.title}`}
                  >
                    <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217zM14.657 2.929a1 1 0 011.414 0A9.972 9.972 0 0119 10a9.972 9.972 0 01-2.929 7.071 1 1 0 01-1.414-1.414A7.971 7.971 0 0017 10c0-2.21-.894-4.208-2.343-5.657a1 1 0 010-1.414zm-2.829 2.828a1 1 0 011.415 0A5.983 5.983 0 0115 10a5.983 5.983 0 01-1.757 4.243 1 1 0 01-1.415-1.415A3.984 3.984 0 0013 10a3.984 3.984 0 00-1.172-2.828 1 1 0 010-1.415z" clipRule="evenodd"/>
                    </svg>
                  </button>
                )}

                <span className="text-xs text-bronze-500 tabular-nums w-12 text-right">
                  {Math.floor(track.duration / 60)}:{String(track.duration % 60).padStart(2, '0')}
                </span>
              </div>
            ))}
          </div>
        )
      )}
    </div>
  )
}
