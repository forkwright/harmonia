import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { apiClient } from '../api/client'
import type { Artist, Album, Track } from '../types'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import { usePlayerStore } from '../stores/playerStore'

type View = 'artists' | 'albums' | 'tracks'

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

  // Load artists on mount
  useEffect(() => {
    loadArtists()
  }, [])

  async function loadArtists() {
    try {
      setLoading(true)
      setError(null)
      const data = await apiClient.getArtists()
      setArtists(data)
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
      const data = await apiClient.getAlbums()
      // Filter by artist
      const artistAlbums = data.filter(album => album.artist === artist.name)
      setAlbums(artistAlbums)
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
      const data = await apiClient.getTracks()
      // Filter by album
      const albumTracks = data.filter(track => track.album === album.title)
      setTracks(albumTracks)
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
              <Button onClick={goBack} variant="secondary">
                ← Back
              </Button>
            )}
            <h1 className="text-3xl font-bold text-bronze-900">
              {view === 'artists' && 'Artists'}
              {view === 'albums' && `${selectedArtist?.name} - Albums`}
              {view === 'tracks' && `${selectedAlbum?.title} - Tracks`}
            </h1>
          </div>
        </div>

        {/* Error state */}
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded mb-4">
            {error}
          </div>
        )}

        {/* Loading state */}
        {loading && (
          <div className="text-center text-bronze-600 py-12">
            Loading...
          </div>
        )}

        {/* Artists view */}
        {!loading && view === 'artists' && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {artists.map(artist => (
              <Card
                key={artist.id}
                onClick={() => loadAlbumsByArtist(artist)}
                className="cursor-pointer hover:shadow-lg transition-shadow"
              >
                <div className="p-4">
                  <h3 className="text-xl font-semibold text-bronze-900 mb-2">
                    {artist.name}
                  </h3>
                  <p className="text-bronze-600 text-sm">
                    {artist.albumCount} albums · {artist.trackCount} tracks
                  </p>
                </div>
              </Card>
            ))}
          </div>
        )}

        {/* Albums view */}
        {!loading && view === 'albums' && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {albums.map(album => (
              <Card
                key={album.id}
                onClick={() => loadTracksByAlbum(album)}
                className="cursor-pointer hover:shadow-lg transition-shadow"
              >
                <div className="p-4">
                  {album.coverArtUrl && (
                    <img
                      src={album.coverArtUrl}
                      alt={album.title}
                      className="w-full aspect-square object-cover rounded mb-3"
                    />
                  )}
                  <h3 className="text-xl font-semibold text-bronze-900 mb-1">
                    {album.title}
                  </h3>
                  <p className="text-bronze-600 text-sm mb-2">
                    {album.artist}
                  </p>
                  <p className="text-bronze-500 text-xs">
                    {album.year && `${album.year} · `}
                    {album.trackCount} tracks · {Math.floor(album.duration / 60)}min
                  </p>
                </div>
              </Card>
            ))}
          </div>
        )}

        {/* Tracks view */}
        {!loading && view === 'tracks' && (
          <div className="space-y-2">
            {tracks.map(track => (
              <Card
                key={track.id}
                onClick={() => handleTrackSelect(track)}
                className="cursor-pointer hover:bg-bronze-100 transition-colors"
              >
                <div className="p-4 flex items-center justify-between">
                  <div className="flex-1">
                    <h3 className="text-lg font-medium text-bronze-900 mb-1">
                      {track.title}
                    </h3>
                    <p className="text-bronze-600 text-sm">
                      {track.artist} · {track.album}
                    </p>
                    <p className="text-bronze-500 text-xs mt-1">
                      {track.format.toUpperCase()} · {(track.sampleRate / 1000).toFixed(1)}kHz/{track.bitDepth}bit
                    </p>
                  </div>
                  <div className="text-bronze-600 text-sm">
                    {Math.floor(track.duration / 60)}:{String(track.duration % 60).padStart(2, '0')}
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}

        {/* Empty state */}
        {!loading && !error && (
          <>
            {view === 'artists' && artists.length === 0 && (
              <div className="text-center text-bronze-600 py-12">
                No artists found
              </div>
            )}
            {view === 'albums' && albums.length === 0 && (
              <div className="text-center text-bronze-600 py-12">
                No albums found for this artist
              </div>
            )}
            {view === 'tracks' && tracks.length === 0 && (
              <div className="text-center text-bronze-600 py-12">
                No tracks found for this album
              </div>
            )}
          </>
        )}
    </div>
  )
}
