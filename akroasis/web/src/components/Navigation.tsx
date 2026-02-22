import { useRef, useState, useEffect, useCallback } from 'react'
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
      }).catch(() => {
        // Track fetch failed — stay on current page
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

  const [menuOpen, setMenuOpen] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)

  const isActive = useCallback((path: string) => {
    if (path === '/audiobooks') return location.pathname.startsWith('/audiobooks')
    return location.pathname === path
  }, [location.pathname])

  function handleLogout() {
    logout()
    navigate('/login')
  }

  function navTo(path: string) {
    navigate(path)
    setMenuOpen(false)
  }

  // Close mobile menu on outside click or Escape
  useEffect(() => {
    if (!menuOpen) return
    function handleClick(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setMenuOpen(false)
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === 'Escape') setMenuOpen(false)
    }
    document.addEventListener('mousedown', handleClick)
    document.addEventListener('keydown', handleKey)
    return () => {
      document.removeEventListener('mousedown', handleClick)
      document.removeEventListener('keydown', handleKey)
    }
  }, [menuOpen])

  const libraryLinks = [
    { path: '/library', label: 'Music' },
    { path: '/audiobooks', label: 'Audiobooks' },
    { path: '/podcasts', label: 'Podcasts' },
  ]

  const toolLinks = [
    { path: '/discover', label: 'Discover' },
    { path: '/queue', label: 'Queue' },
    { path: '/player', label: 'Player' },
    { path: '/settings', label: 'Settings' },
  ]

  return (
    <nav className="bg-bronze-900 text-bronze-50 shadow-lg" ref={menuRef}>
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

          {/* Desktop nav — grouped */}
          <div className="hidden lg:flex items-center gap-2">
            <div className="flex items-center gap-1 bg-bronze-800/30 rounded-lg px-1.5 py-1">
              {libraryLinks.map(({ path, label }) => (
                <Button
                  key={path}
                  variant={isActive(path) ? 'primary' : 'ghost'}
                  size="sm"
                  onClick={() => navTo(path)}
                >
                  {label}
                </Button>
              ))}
            </div>

            <div className="w-px h-6 bg-bronze-700/50" />

            <div className="flex items-center gap-1 bg-bronze-800/30 rounded-lg px-1.5 py-1">
              {toolLinks.map(({ path, label }) => (
                <Button
                  key={path}
                  variant={isActive(path) ? 'primary' : 'ghost'}
                  size="sm"
                  onClick={() => navTo(path)}
                >
                  {label}
                </Button>
              ))}
            </div>

            <Button variant="ghost" size="sm" onClick={handleLogout}>
              Logout
            </Button>
          </div>

          {/* Mobile hamburger */}
          <button
            className="lg:hidden p-2 text-bronze-400 hover:text-bronze-200 transition-colors"
            onClick={() => setMenuOpen(!menuOpen)}
            aria-label={menuOpen ? 'Close menu' : 'Open menu'}
            aria-expanded={menuOpen}
          >
            {menuOpen ? (
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            ) : (
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M4 6h16M4 12h16M4 18h16" />
              </svg>
            )}
          </button>
        </div>
      </div>

      {/* Mobile slide-down menu */}
      {menuOpen && (
        <div className="lg:hidden border-t border-bronze-800 bg-bronze-900 px-4 py-4 space-y-4">
          <div>
            <p className="text-[10px] font-semibold text-bronze-500 uppercase tracking-wider mb-2">Library</p>
            <div className="flex flex-col gap-1">
              {libraryLinks.map(({ path, label }) => (
                <button
                  key={path}
                  onClick={() => navTo(path)}
                  className={`text-left px-3 py-2 rounded-lg text-sm transition-colors ${
                    isActive(path) ? 'bg-bronze-700 text-bronze-100' : 'text-bronze-400 hover:bg-bronze-800 hover:text-bronze-200'
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>

          <div>
            <p className="text-[10px] font-semibold text-bronze-500 uppercase tracking-wider mb-2">Tools</p>
            <div className="flex flex-col gap-1">
              {toolLinks.map(({ path, label }) => (
                <button
                  key={path}
                  onClick={() => navTo(path)}
                  className={`text-left px-3 py-2 rounded-lg text-sm transition-colors ${
                    isActive(path) ? 'bg-bronze-700 text-bronze-100' : 'text-bronze-400 hover:bg-bronze-800 hover:text-bronze-200'
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>

          <div className="border-t border-bronze-800 pt-3">
            <button
              onClick={handleLogout}
              className="text-left px-3 py-2 rounded-lg text-sm text-bronze-400 hover:bg-bronze-800 hover:text-bronze-200 transition-colors w-full"
            >
              Logout
            </button>
          </div>
        </div>
      )}
    </nav>
  )
}
