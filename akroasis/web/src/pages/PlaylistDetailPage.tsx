// Playlist detail — track list with drag-to-reorder
import { useState, useEffect, useCallback, useRef } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { usePlaylistStore } from '../stores/playlistStore'
import { usePlayerStore } from '../stores/playerStore'
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer'
import { HeartButton } from '../components/HeartButton'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import { apiClient } from '../api/client'
import type { SearchResult } from '../types'
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import type { DragEndEvent } from '@dnd-kit/core'
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import type { Track } from '../types'

interface SortablePlaylistTrackProps {
  readonly track: Track
  readonly index: number
  readonly onPlay: (track: Track) => void
  readonly onRemove: (trackId: number) => void
}

function SortablePlaylistTrack({ track, index, onPlay, onRemove }: SortablePlaylistTrackProps) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id: track.id })

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  }

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="flex items-center gap-3 p-3 rounded-lg bg-surface-raised/60 hover:bg-accent-subtle transition-colors"
    >
      <button
        {...attributes}
        {...listeners}
        className="cursor-grab active:cursor-grabbing text-theme-tertiary hover:text-theme-secondary p-1"
        aria-label="Drag to reorder"
      >
        <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
          <path d="M7 2a2 2 0 1 0 .001 4.001A2 2 0 0 0 7 2zm0 6a2 2 0 1 0 .001 4.001A2 2 0 0 0 7 8zm0 6a2 2 0 1 0 .001 4.001A2 2 0 0 0 7 14zm6-8a2 2 0 1 0-.001-4.001A2 2 0 0 0 13 6zm0 2a2 2 0 1 0 .001 4.001A2 2 0 0 0 13 8zm0 6a2 2 0 1 0 .001 4.001A2 2 0 0 0 13 14z"/>
        </svg>
      </button>

      <div className="flex-1 min-w-0">
        <div className="flex items-baseline gap-2">
          <span className="text-theme-tertiary text-sm font-mono">
            {(index + 1).toString().padStart(2, '0')}
          </span>
          <h3 className="text-theme-primary font-medium truncate">{track.title}</h3>
        </div>
        <p className="text-sm text-theme-tertiary mt-0.5">
          {track.artist} • {track.album}
        </p>
      </div>

      <HeartButton trackId={track.id} />

      <button
        onClick={() => onPlay(track)}
        className="text-theme-tertiary hover:text-theme-secondary p-2"
        title="Play now"
      >
        <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
        </svg>
      </button>

      <button
        onClick={() => onRemove(track.id)}
        className="text-theme-muted hover:text-red-400 p-2"
        title="Remove from playlist"
      >
        <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd"/>
        </svg>
      </button>
    </div>
  )
}

// --- Add Songs Modal ---
function AddSongsModal({
  existingTrackIds,
  onAdd,
  onClose,
}: {
  existingTrackIds: Set<number>
  onAdd: (trackId: number) => Promise<void>
  onClose: () => void
}) {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<SearchResult[]>([])
  const [searching, setSearching] = useState(false)
  const [addedIds, setAddedIds] = useState<Set<number>>(new Set())
  const inputRef = useRef<HTMLInputElement>(null)
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined)

  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current)
    if (!query.trim()) {
      setResults([])
      return
    }
    debounceRef.current = setTimeout(async () => {
      setSearching(true)
      try {
        const data = await apiClient.search(query, 30)
        setResults(data)
      } catch {
        // Non-critical
      } finally {
        setSearching(false)
      }
    }, 300)
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current) }
  }, [query])

  const handleAdd = async (trackId: number) => {
    await onAdd(trackId)
    setAddedIds((prev) => new Set([...prev, trackId]))
  }

  const formatDuration = (seconds?: number) => {
    if (!seconds) return ''
    const m = Math.floor(seconds / 60)
    const s = Math.floor(seconds % 60)
    return `${m}:${s.toString().padStart(2, '0')}`
  }

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[10vh] bg-black/50 backdrop-blur-sm" onClick={onClose}>
      <div
        className="w-full max-w-lg mx-4 bg-surface-base rounded-xl shadow-2xl border border-theme-subtle max-h-[70vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="p-4 border-b border-theme-subtle">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold text-theme-primary">Add Songs</h2>
            <button onClick={onClose} className="text-theme-muted hover:text-theme-secondary p-1">
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clipRule="evenodd"/>
              </svg>
            </button>
          </div>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search for songs..."
            className="w-full bg-surface-raised text-theme-primary rounded-lg px-3 py-2 text-sm placeholder-theme-muted focus:outline-none focus:ring-1 focus:ring-accent"
          />
        </div>

        <div className="flex-1 overflow-y-auto p-2">
          {searching && (
            <div className="text-center py-4 text-theme-tertiary text-sm">Searching...</div>
          )}

          {!searching && query.trim() && results.length === 0 && (
            <div className="text-center py-4 text-theme-tertiary text-sm">No results</div>
          )}

          {!query.trim() && (
            <div className="text-center py-8 text-theme-muted text-sm">Type to search your library</div>
          )}

          {results.map((result) => {
            const alreadyIn = existingTrackIds.has(result.trackId) || addedIds.has(result.trackId)
            return (
              <div
                key={result.trackId}
                className="flex items-center gap-3 p-2.5 rounded-lg hover:bg-accent-subtle transition-colors"
              >
                <div className="flex-1 min-w-0">
                  <p className="text-sm text-theme-primary font-medium truncate">{result.title}</p>
                  <p className="text-xs text-theme-tertiary truncate">
                    {result.artist} {result.album ? `• ${result.album}` : ''}
                    {result.durationSeconds ? ` • ${formatDuration(result.durationSeconds)}` : ''}
                  </p>
                </div>
                <button
                  onClick={() => void handleAdd(result.trackId)}
                  disabled={alreadyIn}
                  className={`flex-shrink-0 p-1.5 rounded-full transition-colors ${
                    alreadyIn
                      ? 'text-green-400 cursor-default'
                      : 'text-theme-muted hover:text-accent hover:bg-accent-subtle'
                  }`}
                  title={alreadyIn ? 'Already added' : 'Add to playlist'}
                >
                  {alreadyIn ? (
                    <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd"/>
                    </svg>
                  ) : (
                    <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z" clipRule="evenodd"/>
                    </svg>
                  )}
                </button>
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}

export function PlaylistDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const playlistId = Number(id)

  const { playlists, getPlaylistTracks, addTrackToPlaylist, removeTrackFromPlaylist, reorderPlaylistTracks, loadPlaylists } = usePlaylistStore()
  const { setQueue, setCurrentTrack } = usePlayerStore()
  const { playTrack } = useWebAudioPlayer()

  const playlist = playlists.find((p) => p.id === playlistId)
  const [tracks, setTracks] = useState<Track[]>([])
  const [loading, setLoading] = useState(true)
  const [showAddSongs, setShowAddSongs] = useState(false)

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
  )

  const loadTracks = useCallback(async () => {
    setLoading(true)
    try {
      const result = await getPlaylistTracks(playlistId)
      setTracks(result)
    } catch {
      // Non-critical
    } finally {
      setLoading(false)
    }
  }, [playlistId, getPlaylistTracks])

  useEffect(() => {
    if (playlists.length === 0) void loadPlaylists()
    void loadTracks()
  }, [loadTracks, playlists.length, loadPlaylists])

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event
    if (over && active.id !== over.id) {
      const oldIndex = tracks.findIndex((t) => t.id === active.id)
      const newIndex = tracks.findIndex((t) => t.id === over.id)
      const reordered = arrayMove(tracks, oldIndex, newIndex)
      setTracks(reordered)
      void reorderPlaylistTracks(playlistId, reordered.map((t) => t.id))
    }
  }

  const handlePlayTrack = (track: Track) => {
    setQueue(tracks)
    setCurrentTrack(track)
    playTrack(track)
  }

  const handlePlayAll = () => {
    if (tracks.length > 0) {
      setQueue(tracks)
      setCurrentTrack(tracks[0])
      playTrack(tracks[0])
    }
  }

  const handleRemoveTrack = async (trackId: number) => {
    await removeTrackFromPlaylist(playlistId, trackId)
    setTracks((prev) => prev.filter((t) => t.id !== trackId))
  }

  const handleAddTrack = async (trackId: number) => {
    await addTrackToPlaylist(playlistId, trackId)
    // Reload tracks to get the full Track object
    void loadTracks()
  }

  if (!playlist && !loading) {
    return (
      <div className="container mx-auto p-4 max-w-4xl">
        <Card>
          <p className="text-theme-tertiary text-center py-8">Playlist not found</p>
          <div className="text-center">
            <Button variant="ghost" size="sm" onClick={() => navigate('/playlists')}>
              Back to Playlists
            </Button>
          </div>
        </Card>
      </div>
    )
  }

  return (
    <div className="container mx-auto p-4 max-w-4xl">
      <Card>
        <div className="flex items-center justify-between mb-6">
          <div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => navigate('/playlists')}
                className="text-theme-tertiary hover:text-theme-secondary"
                aria-label="Back to playlists"
              >
                <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M9.707 16.707a1 1 0 01-1.414 0l-6-6a1 1 0 010-1.414l6-6a1 1 0 011.414 1.414L5.414 9H17a1 1 0 110 2H5.414l4.293 4.293a1 1 0 010 1.414z" clipRule="evenodd"/>
                </svg>
              </button>
              <h1 className="text-2xl font-serif font-semibold" style={{ color: 'rgb(var(--text-primary))' }}>{playlist?.name ?? 'Loading...'}</h1>
            </div>
            {playlist?.description && (
              <p className="text-sm text-theme-tertiary mt-1 ml-7">{playlist.description}</p>
            )}
            <p className="text-sm text-theme-tertiary mt-0.5 ml-7">{tracks.length} tracks</p>
          </div>
          <div className="flex gap-2">
            <Button variant="ghost" size="sm" onClick={() => setShowAddSongs(true)}>
              <svg className="w-4 h-4 mr-2" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z" clipRule="evenodd"/>
              </svg>
              Add Songs
            </Button>
            <Button variant="ghost" size="sm" onClick={handlePlayAll} disabled={tracks.length === 0}>
              <svg className="w-4 h-4 mr-2" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
              </svg>
              Play All
            </Button>
          </div>
        </div>

        {loading ? (
          <div className="text-center py-8 text-theme-tertiary">Loading tracks...</div>
        ) : tracks.length === 0 ? (
          <div className="text-center py-12">
            <p className="text-theme-tertiary">No tracks in this playlist</p>
            <p className="text-sm text-theme-muted mt-1">Add tracks from the library</p>
          </div>
        ) : (
          <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
            <SortableContext items={tracks.map((t) => t.id)} strategy={verticalListSortingStrategy}>
              <div className="space-y-2">
                {tracks.map((track, index) => (
                  <SortablePlaylistTrack
                    key={track.id}
                    track={track}
                    index={index}
                    onPlay={handlePlayTrack}
                    onRemove={(trackId) => void handleRemoveTrack(trackId)}
                  />
                ))}
              </div>
            </SortableContext>
          </DndContext>
        )}
      </Card>

      {showAddSongs && (
        <AddSongsModal
          existingTrackIds={new Set(tracks.map((t) => t.id))}
          onAdd={handleAddTrack}
          onClose={() => setShowAddSongs(false)}
        />
      )}
    </div>
  )
}
