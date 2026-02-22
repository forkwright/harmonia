import { useRef, useState, useEffect, useCallback } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { SearchDropdown } from './SearchDropdown'
import { useAuthStore } from '../stores/authStore'
import { useSearchStore } from '../stores/searchStore'
import { usePlayerStore } from '../stores/playerStore'
import { useDebounce } from '../hooks/useDebounce'
import { apiClient } from '../api/client'
import type { UnifiedSearchResult } from '../types'

const NAV_ITEMS = [
  { path: '/library', label: 'Music', icon: 'M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z' },
  { path: '/audiobooks', label: 'Books', icon: 'M9 4.804A7.968 7.968 0 005.5 4c-1.255 0-2.443.29-3.5.804v10A7.969 7.969 0 015.5 14c1.669 0 3.218.51 4.5 1.385A7.962 7.962 0 0114.5 14c1.255 0 2.443.29 3.5.804v-10A7.968 7.968 0 0014.5 4c-1.255 0-2.443.29-3.5.804V14a1 1 0 11-2 0V4.804z' },
  { path: '/podcasts', label: 'Podcasts', icon: 'M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217zM14.657 2.929a1 1 0 011.414 0A9.972 9.972 0 0119 10a9.972 9.972 0 01-2.929 7.071 1 1 0 01-1.414-1.414A7.971 7.971 0 0017 10c0-2.21-.894-4.208-2.343-5.657a1 1 0 010-1.414zm-2.829 2.828a1 1 0 011.415 0A5.983 5.983 0 0115 10a5.983 5.983 0 01-1.757 4.243 1 1 0 01-1.415-1.415A3.984 3.984 0 0013 10a3.984 3.984 0 00-1.172-2.828 1 1 0 010-1.415z' },
  { path: '/discover', label: 'Discover', icon: 'M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z' },
]

const TOOL_ITEMS = [
  { path: '/playlists', label: 'Playlists' },
  { path: '/queue', label: 'Queue' },
  { path: '/player', label: 'Player' },
  { path: '/settings', label: 'Settings' },
]

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

  // Cmd+K / Ctrl+K to focus search
  useEffect(() => {
    function handleGlobalKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        inputRef.current?.focus()
      }
    }
    document.addEventListener('keydown', handleGlobalKey)
    return () => document.removeEventListener('keydown', handleGlobalKey)
  }, [])

  const handleSelect = useCallback((result: UnifiedSearchResult) => {
    clear()
    if (result.type === 'track') {
      void apiClient.getTrack(result.id).then((track) => {
        setCurrentTrack(track)
        setIsPlaying(true)
        navigate('/player')
      }).catch(() => {})
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

  return (
    <nav className="sticky top-0 z-40 bg-neutral-950/90 backdrop-blur-md border-b border-bronze-800/30" ref={menuRef}>
      <div className="max-w-7xl mx-auto px-4">
        <div className="flex items-center justify-between h-14 gap-4">

          {/* Brand */}
          <button
            onClick={() => navTo('/library')}
            className="flex items-center gap-2 shrink-0 group"
          >
            <span className="text-lg font-bold text-bronze-200 group-hover:text-bronze-100 transition-colors">
              Akroasis
            </span>
          </button>

          {/* Nav pills — desktop */}
          <div className="hidden md:flex items-center gap-1">
            {NAV_ITEMS.map(({ path, label, icon }) => (
              <button
                key={path}
                onClick={() => navTo(path)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm transition-colors ${
                  isActive(path)
                    ? 'bg-bronze-800 text-bronze-100'
                    : 'text-bronze-400 hover:text-bronze-200 hover:bg-bronze-800/50'
                }`}
              >
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                  <path d={icon} fillRule="evenodd" clipRule="evenodd" />
                </svg>
                {label}
              </button>
            ))}

            <div className="w-px h-5 bg-bronze-800/50 mx-1" />

            {TOOL_ITEMS.map(({ path, label }) => (
              <button
                key={path}
                onClick={() => navTo(path)}
                className={`px-2.5 py-1.5 rounded-lg text-sm transition-colors ${
                  isActive(path)
                    ? 'bg-bronze-800 text-bronze-100'
                    : 'text-bronze-500 hover:text-bronze-300 hover:bg-bronze-800/50'
                }`}
              >
                {label}
              </button>
            ))}
          </div>

          {/* Search */}
          <div ref={containerRef} className="relative flex-1 max-w-xs">
            <div className="relative">
              <svg
                className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-bronze-600 pointer-events-none"
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
                className="w-full pl-8 pr-12 py-1.5 bg-bronze-900/50 border border-bronze-800/50 rounded-lg text-sm text-bronze-200 placeholder-bronze-600 focus:outline-none focus:border-bronze-600 focus:bg-bronze-900 transition-all"
                aria-label="Search across music, audiobooks, and podcasts"
                role="combobox"
                aria-expanded={isOpen}
                aria-haspopup="listbox"
              />
              <kbd className="absolute right-2.5 top-1/2 -translate-y-1/2 hidden sm:inline-flex items-center gap-0.5 px-1.5 py-0.5 text-[10px] text-bronze-600 bg-bronze-800/50 rounded border border-bronze-700/30 pointer-events-none">
                ⌘K
              </kbd>
            </div>
            {isOpen && (
              <SearchDropdown
                results={results}
                selectedIndex={selectedIndex}
                onSelect={handleSelect}
              />
            )}
          </div>

          {/* Desktop logout */}
          <button
            onClick={handleLogout}
            className="hidden md:block text-sm text-bronze-600 hover:text-bronze-400 transition-colors"
          >
            Logout
          </button>

          {/* Mobile hamburger */}
          <button
            className="md:hidden p-2 text-bronze-500 hover:text-bronze-300 transition-colors"
            onClick={() => setMenuOpen(!menuOpen)}
            aria-label={menuOpen ? 'Close menu' : 'Open menu'}
            aria-expanded={menuOpen}
          >
            {menuOpen ? (
              <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            ) : (
              <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M4 6h16M4 12h16M4 18h16" />
              </svg>
            )}
          </button>
        </div>
      </div>

      {/* Mobile menu */}
      {menuOpen && (
        <div className="md:hidden border-t border-bronze-800/30 bg-neutral-950/95 backdrop-blur-md px-4 py-3 space-y-1">
          {NAV_ITEMS.map(({ path, label }) => (
            <button
              key={path}
              onClick={() => navTo(path)}
              className={`w-full text-left px-3 py-2.5 rounded-lg text-sm transition-colors ${
                isActive(path) ? 'bg-bronze-800 text-bronze-100' : 'text-bronze-400 hover:bg-bronze-800/50'
              }`}
            >
              {label}
            </button>
          ))}

          <div className="h-px bg-bronze-800/30 my-2" />

          {TOOL_ITEMS.map(({ path, label }) => (
            <button
              key={path}
              onClick={() => navTo(path)}
              className={`w-full text-left px-3 py-2.5 rounded-lg text-sm transition-colors ${
                isActive(path) ? 'bg-bronze-800 text-bronze-100' : 'text-bronze-500 hover:bg-bronze-800/50'
              }`}
            >
              {label}
            </button>
          ))}

          <div className="h-px bg-bronze-800/30 my-2" />

          <button
            onClick={handleLogout}
            className="w-full text-left px-3 py-2.5 rounded-lg text-sm text-bronze-500 hover:bg-bronze-800/50"
          >
            Logout
          </button>
        </div>
      )}
    </nav>
  )
}
