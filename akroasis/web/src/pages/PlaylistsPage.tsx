// Playlist listing and creation
import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { usePlaylistStore } from '../stores/playlistStore'
import { Card } from '../components/Card'
import { Button } from '../components/Button'

export function PlaylistsPage() {
  const navigate = useNavigate()
  const { playlists, loading, error, loadPlaylists, createPlaylist, deletePlaylist } = usePlaylistStore()
  const [showCreate, setShowCreate] = useState(false)
  const [newName, setNewName] = useState('')
  const [newDescription, setNewDescription] = useState('')

  useEffect(() => {
    void loadPlaylists()
  }, [loadPlaylists])

  const handleCreate = async () => {
    if (!newName.trim()) return
    try {
      const playlist = await createPlaylist(newName.trim(), newDescription.trim() || undefined)
      setNewName('')
      setNewDescription('')
      setShowCreate(false)
      navigate(`/playlists/${playlist.id}`)
    } catch {
      // Error handled by store
    }
  }

  const handleDelete = async (e: React.MouseEvent, id: number) => {
    e.stopPropagation()
    if (confirm('Delete this playlist?')) {
      await deletePlaylist(id)
    }
  }

  const formatDuration = (ms: number) => {
    const hours = Math.floor(ms / 3600000)
    const minutes = Math.floor((ms % 3600000) / 60000)
    return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`
  }

  return (
    <div className="container mx-auto p-4 max-w-4xl">
      <Card>
        <div className="flex items-center justify-between mb-6">
          <h1 className="text-2xl font-serif font-semibold" style={{ color: 'rgb(var(--text-primary))' }}>Playlists</h1>
          <Button variant="ghost" size="sm" onClick={() => setShowCreate(!showCreate)}>
            <svg className="w-4 h-4 mr-2" fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z" clipRule="evenodd"/>
            </svg>
            New Playlist
          </Button>
        </div>

        {showCreate && (
          <div className="mb-6 p-4 bg-surface-raised/60 rounded-lg space-y-3">
            <input
              type="text"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="Playlist name"
              className="w-full bg-surface-raised text-theme-primary rounded px-3 py-2 text-sm placeholder-theme-muted focus:outline-none focus:ring-1 focus:ring-accent"
              onKeyDown={(e) => e.key === 'Enter' && void handleCreate()}
            />
            <input
              type="text"
              value={newDescription}
              onChange={(e) => setNewDescription(e.target.value)}
              placeholder="Description (optional)"
              className="w-full bg-surface-raised text-theme-primary rounded px-3 py-2 text-sm placeholder-theme-muted focus:outline-none focus:ring-1 focus:ring-accent"
            />
            <div className="flex gap-2">
              <Button variant="ghost" size="sm" onClick={() => void handleCreate()} disabled={!newName.trim()}>
                Create
              </Button>
              <Button variant="ghost" size="sm" onClick={() => setShowCreate(false)}>
                Cancel
              </Button>
            </div>
          </div>
        )}

        {loading && (
          <div className="text-center py-8 text-theme-tertiary">Loading playlists...</div>
        )}

        {error && (
          <div className="text-center py-4 text-red-400">{error}</div>
        )}

        {!loading && playlists.length === 0 ? (
          <div className="text-center py-12">
            <svg className="w-16 h-16 text-theme-muted mx-auto mb-4" fill="currentColor" viewBox="0 0 20 20">
              <path d="M9 2a1 1 0 000 2h2a1 1 0 100-2H9z"/>
              <path fillRule="evenodd" d="M4 5a2 2 0 012-2 3 3 0 003 3h2a3 3 0 003-3 2 2 0 012 2v11a2 2 0 01-2 2H6a2 2 0 01-2-2V5zm3 4a1 1 0 000 2h.01a1 1 0 100-2H7zm3 0a1 1 0 000 2h3a1 1 0 100-2h-3zm-3 4a1 1 0 100 2h.01a1 1 0 100-2H7zm3 0a1 1 0 100 2h3a1 1 0 100-2h-3z" clipRule="evenodd"/>
            </svg>
            <p className="text-theme-tertiary">No playlists yet</p>
            <p className="text-sm text-theme-muted mt-1">Create a playlist to start organizing your music</p>
          </div>
        ) : (
          <div className="space-y-2">
            {playlists.map((playlist) => (
              <div
                key={playlist.id}
                onClick={() => navigate(`/playlists/${playlist.id}`)}
                className="flex items-center justify-between p-4 rounded-lg bg-surface-raised/60 hover:bg-accent-subtle cursor-pointer transition-colors"
              >
                <div>
                  <h3 className="text-theme-primary font-medium">{playlist.name}</h3>
                  <p className="text-sm text-theme-tertiary mt-0.5">
                    {playlist.trackCount} tracks • {formatDuration(playlist.totalDuration)}
                    {playlist.description && ` • ${playlist.description}`}
                  </p>
                </div>
                <button
                  onClick={(e) => void handleDelete(e, playlist.id)}
                  className="text-theme-muted hover:text-red-400 p-2"
                  title="Delete playlist"
                >
                  <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z" clipRule="evenodd"/>
                  </svg>
                </button>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  )
}
