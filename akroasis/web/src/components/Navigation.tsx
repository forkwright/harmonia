import { useRef, useEffect, useCallback } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { Button } from './Button'
import { SearchDropdown } from './SearchDropdown'
import { useAuthStore } from '../stores/authStore'
import { useSearchStore } from '../stores/searchStore'
import { usePlayerStore } from '../stores/playerStore'
import { useDebounce } from '../hooks/useDebounce'
import { apiClient } from '../api/client'
import type { UnifiedSearchResult } from '../types'

export function Navigation() {
  const location = useLocation()
  const navigate = useNavigate()
  const logout = useAuthStore((state) => state.logout)
  const { query, results, isOpen, selectedIndex, setQuery, search, setOpen, setSelectedIndex, clear } = useSearchStore()
  const setCurrentTrack = usePlayerStore((s) => s.setCurrentTrack)
  const setIsPlaying = usePlayerStore((s) => s.setIsPlaying)

  const debouncedQuery = useDebounce(query, 300)
  const containerRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (debouncedQuery.trim()) {
      void search(debouncedQuery)
    } else {
      setOpen(false)
    }
  }, [debouncedQuery, search, setOpen])

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [setOpen])

  const handleSelect = useCallback((result: UnifiedSearchResult) => {
    clear()
    if (result.type === 'track') {
      void apiClient.getTrack(result.id).then((track) => {
        setCurrentTrack(track)
        setIsPlaying(true)
        navigate('/player')
      })
    } else if (result.type === 'audiobook') {
      navigate(`/audiobooks/play/${result.id}`)
    } else if (result.type === 'podcast') {
      navigate('/podcasts')
    }
  }, [clear, navigate, setCurrentTrack, setIsPlaying])

  function handleKeyDown(e: React.KeyboardEvent) {
    if (!isOpen || results.length === 0) {
      if (e.key === 'Escape') {
        clear()
        inputRef.current?.blur()
      }
      return
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        setSelectedIndex(Math.min(selectedIndex + 1, results.length - 1))
        break
      case 'ArrowUp':
        e.preventDefault()
        setSelectedIndex(Math.max(selectedIndex - 1, -1))
        break
      case 'Enter':
        e.preventDefault()
        if (selectedIndex >= 0 && selectedIndex < results.length) {
          handleSelect(results[selectedIndex])
        }
        break
      case 'Escape':
        e.preventDefault()
        clear()
        inputRef.current?.blur()
        break
    }
  }

  const isAudiobooks = location.pathname.startsWith('/audiobooks')
  const isLibrary = location.pathname === '/library'
  const isPodcasts = location.pathname === '/podcasts'
  const isDiscover = location.pathname === '/discover'
  const isPlayer = location.pathname === '/player'
  const isQueue = location.pathname === '/queue'
  const isSettings = location.pathname === '/settings'

  function handleLogout() {
    logout()
    navigate('/login')
  }

  return (
    <nav className="bg-bronze-900 text-bronze-50 shadow-lg">
      <div className="max-w-7xl mx-auto px-4">
        <div className="flex items-center justify-between h-16 gap-4">
          <div className="flex items-center gap-2 shrink-0">
            <h1 className="text-xl font-bold">Akroasis</h1>
            <span className="text-bronze-400 text-sm hidden sm:inline">Ἀκρόασις</span>
          </div>

          <div ref={containerRef} className="relative flex-1 max-w-xs">
            <div className="relative">
              <svg
                className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-bronze-500 pointer-events-none"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
              <input
                ref={inputRef}
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={handleKeyDown}
                onFocus={() => { if (results.length > 0) setOpen(true) }}
                placeholder="Search..."
                className="w-full pl-8 pr-3 py-1.5 bg-bronze-800 border border-bronze-700 rounded-lg text-sm text-bronze-100 placeholder-bronze-500 focus:outline-none focus:border-bronze-500 transition-colors"
                aria-label="Search across music, audiobooks, and podcasts"
                role="combobox"
                aria-expanded={isOpen}
                aria-haspopup="listbox"
              />
            </div>
            {isOpen && (
              <SearchDropdown
                results={results}
                selectedIndex={selectedIndex}
                onSelect={handleSelect}
              />
            )}
          </div>

          <div className="flex items-center gap-4">
            <Button
              variant={isAudiobooks ? 'primary' : 'secondary'}
              onClick={() => navigate('/audiobooks')}
              className="min-w-24"
            >
              Audiobooks
            </Button>
            <Button
              variant={isLibrary ? 'primary' : 'secondary'}
              onClick={() => navigate('/library')}
              className="min-w-24"
            >
              Music
            </Button>
            <Button
              variant={isPodcasts ? 'primary' : 'secondary'}
              onClick={() => navigate('/podcasts')}
              className="min-w-24"
            >
              Podcasts
            </Button>
            <Button
              variant={isDiscover ? 'primary' : 'secondary'}
              onClick={() => navigate('/discover')}
              className="min-w-24"
            >
              Discover
            </Button>
            <Button
              variant={isQueue ? 'primary' : 'secondary'}
              onClick={() => navigate('/queue')}
              className="min-w-24"
            >
              Queue
            </Button>
            <Button
              variant={isPlayer ? 'primary' : 'secondary'}
              onClick={() => navigate('/player')}
              className="min-w-24"
            >
              Player
            </Button>
            <Button
              variant={isSettings ? 'primary' : 'secondary'}
              onClick={() => navigate('/settings')}
              className="min-w-24"
            >
              Settings
            </Button>
            <Button variant="secondary" onClick={handleLogout}>
              Logout
            </Button>
          </div>
        </div>
      </div>
    </nav>
  )
}
