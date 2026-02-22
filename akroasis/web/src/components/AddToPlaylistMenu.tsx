// Dropdown menu to add a track to a playlist
import { useState, useRef, useEffect } from 'react'
import { usePlaylistStore } from '../stores/playlistStore'

interface AddToPlaylistMenuProps {
  readonly trackId: number
}

export function AddToPlaylistMenu({ trackId }: AddToPlaylistMenuProps) {
  const [open, setOpen] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)
  const { playlists, loadPlaylists, addTrackToPlaylist } = usePlaylistStore()

  useEffect(() => {
    if (open && playlists.length === 0) {
      void loadPlaylists()
    }
  }, [open, playlists.length, loadPlaylists])

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    if (open) document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [open])

  const handleAdd = async (playlistId: number) => {
    await addTrackToPlaylist(playlistId, trackId)
    setOpen(false)
  }

  return (
    <div ref={menuRef} className="relative">
      <button
        onClick={(e) => {
          e.stopPropagation()
          setOpen(!open)
        }}
        className="text-bronze-600 hover:text-bronze-300 p-1"
        title="Add to playlist"
        aria-label="Add to playlist"
      >
        <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
          <path fillRule="evenodd" d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z" clipRule="evenodd"/>
        </svg>
      </button>

      {open && (
        <div className="absolute right-0 top-full mt-1 z-50 min-w-[180px] bg-bronze-900 border border-bronze-700 rounded-lg shadow-lg py-1">
          {playlists.length === 0 ? (
            <div className="px-3 py-2 text-sm text-bronze-500">No playlists</div>
          ) : (
            playlists.map((playlist) => (
              <button
                key={playlist.id}
                onClick={(e) => {
                  e.stopPropagation()
                  void handleAdd(playlist.id)
                }}
                className="w-full text-left px-3 py-2 text-sm text-bronze-300 hover:bg-bronze-800 transition-colors"
              >
                {playlist.name}
              </button>
            ))
          )}
        </div>
      )}
    </div>
  )
}
