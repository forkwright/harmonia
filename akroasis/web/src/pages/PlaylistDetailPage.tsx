// Playlist detail — track list with drag-to-reorder
import { useState, useEffect, useCallback } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { usePlaylistStore } from '../stores/playlistStore'
import { usePlayerStore } from '../stores/playerStore'
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer'
import { HeartButton } from '../components/HeartButton'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
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

export function PlaylistDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const playlistId = Number(id)

  const { playlists, getPlaylistTracks, removeTrackFromPlaylist, reorderPlaylistTracks, loadPlaylists } = usePlaylistStore()
  const { setQueue, setCurrentTrack } = usePlayerStore()
  const { playTrack } = useWebAudioPlayer()

  const playlist = playlists.find((p) => p.id === playlistId)
  const [tracks, setTracks] = useState<Track[]>([])
  const [loading, setLoading] = useState(true)

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
          <Button variant="ghost" size="sm" onClick={handlePlayAll} disabled={tracks.length === 0}>
            <svg className="w-4 h-4 mr-2" fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
            </svg>
            Play All
          </Button>
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
    </div>
  )
}
