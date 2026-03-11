import { useState, useEffect, useCallback, useMemo, useRef } from 'react'
import { useNavigate } from 'react-router-dom'
import { useShallow } from 'zustand/react/shallow'
import { isLastfmConfigured } from '../api/lastfm'
import type { Track, Artist } from '../types'
import { authenticateUrl } from '../api/client'
import { Button } from '../components/Button'
import { usePlayerStore } from '../stores/playerStore'
import { useRadioStore } from '../stores/radioStore'
import { useLibraryStore, type LibraryView, type SortField, type SortDirection } from '../stores/libraryStore'
import { HeartButton } from '../components/HeartButton'
import { QualityDot, getSourceTier } from '../components/SignalPath'
import { useArtworkViewer } from '../stores/artworkViewerStore'
import { useListeningProfileStore } from '../stores/listeningProfileStore'

// ─── Skeleton Components ────────────────────────────────────────

function Skeleton({ className = '' }: { className?: string }) {
  return <div className={`animate-pulse bg-accent-subtle rounded ${className}`} />
}

function ArtistCardSkeleton() {
  return (
    <div className="bg-surface-raised border border-theme-subtle rounded-xl p-4 flex items-center gap-4">
      <Skeleton className="w-14 h-14 rounded-full flex-shrink-0" />
      <div className="flex-1">
        <Skeleton className="h-5 w-3/4 mb-2" />
        <Skeleton className="h-3 w-1/2" />
      </div>
    </div>
  )
}

function AlbumCardSkeleton() {
  return (
    <div className="bg-surface-raised border border-theme-subtle rounded-xl overflow-hidden">
      <Skeleton className="w-full aspect-square rounded-none" />
      <div className="p-3">
        <Skeleton className="h-4 w-3/4 mb-2" />
        <Skeleton className="h-3 w-1/2 mb-1" />
        <Skeleton className="h-3 w-2/3" />
      </div>
    </div>
  )
}

function TrackRowSkeleton() {
  return (
    <div className="flex items-center gap-4 px-4 py-3">
      <Skeleton className="w-6 h-5" />
      <div className="flex-1">
        <Skeleton className="h-5 w-1/2 mb-2" />
        <Skeleton className="h-4 w-1/3" />
      </div>
      <Skeleton className="w-20 h-4" />
    </div>
  )
}

function EmptyState({ icon, title, subtitle }: { icon: React.ReactNode; title: string; subtitle: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-20">
      <div className="w-20 h-20 rounded-2xl bg-surface-raised flex items-center justify-center mb-4">
        {icon}
      </div>
      <p className="text-theme-secondary text-lg">{title}</p>
      <p className="text-theme-muted text-sm mt-1">{subtitle}</p>
    </div>
  )
}

// ─── Icons ──────────────────────────────────────────────────────

function SearchIcon({ className = 'w-4 h-4' }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
    </svg>
  )
}

function SortIcon({ className = 'w-4 h-4' }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M3 7.5L7.5 3m0 0L12 7.5M7.5 3v13.5m13.5 0L16.5 21m0 0L12 16.5m4.5 4.5V7.5" />
    </svg>
  )
}

function GridIcon({ className = 'w-4 h-4' }: { className?: string }) {
  return (
    <svg className={className} fill="currentColor" viewBox="0 0 20 20">
      <path fillRule="evenodd" d="M4.25 2A2.25 2.25 0 002 4.25v2.5A2.25 2.25 0 004.25 9h2.5A2.25 2.25 0 009 6.75v-2.5A2.25 2.25 0 006.75 2h-2.5zm0 9A2.25 2.25 0 002 13.25v2.5A2.25 2.25 0 004.25 18h2.5A2.25 2.25 0 009 15.75v-2.5A2.25 2.25 0 006.75 11h-2.5zm9-9A2.25 2.25 0 0011 4.25v2.5A2.25 2.25 0 0013.25 9h2.5A2.25 2.25 0 0018 6.75v-2.5A2.25 2.25 0 0015.75 2h-2.5zm0 9A2.25 2.25 0 0011 13.25v2.5A2.25 2.25 0 0013.25 18h2.5A2.25 2.25 0 0018 15.75v-2.5A2.25 2.25 0 0015.75 11h-2.5z" clipRule="evenodd" />
    </svg>
  )
}

function ListIcon({ className = 'w-4 h-4' }: { className?: string }) {
  return (
    <svg className={className} fill="currentColor" viewBox="0 0 20 20">
      <path fillRule="evenodd" d="M2 4.75A.75.75 0 012.75 4h14.5a.75.75 0 010 1.5H2.75A.75.75 0 012 4.75zm0 10.5a.75.75 0 01.75-.75h14.5a.75.75 0 010 1.5H2.75a.75.75 0 01-.75-.75zM2 10a.75.75 0 01.75-.75h14.5a.75.75 0 010 1.5H2.75A.75.75 0 012 10z" clipRule="evenodd" />
    </svg>
  )
}

function ChevronIcon({ className = 'w-4 h-4' }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
    </svg>
  )
}

function MusicNoteIcon({ className = 'w-10 h-10' }: { className?: string }) {
  return (
    <svg className={className} fill="currentColor" viewBox="0 0 20 20">
      <path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/>
    </svg>
  )
}

function PersonIcon({ className = 'w-6 h-6' }: { className?: string }) {
  return (
    <svg className={className} fill="currentColor" viewBox="0 0 20 20">
      <path d="M10 8a3 3 0 100-6 3 3 0 000 6zM3.465 14.493a1.23 1.23 0 00.41 1.412A9.957 9.957 0 0010 18c2.31 0 4.438-.784 6.131-2.1.43-.333.604-.903.408-1.41a7.002 7.002 0 00-13.074.003z" />
    </svg>
  )
}

// ─── View Tab Bar ───────────────────────────────────────────────

const VIEW_TABS: { id: LibraryView; label: string; count?: string }[] = [
  { id: 'artists', label: 'Artists' },
  { id: 'albums', label: 'Albums' },
  { id: 'tracks', label: 'Tracks' },
  { id: 'genres', label: 'Genres' },
]

function ViewTabs({ current, onChange }: { current: LibraryView; onChange: (v: LibraryView) => void }) {
  return (
    <div className="flex gap-1 border-b border-theme-subtle">
      {VIEW_TABS.map(tab => (
        <button
          key={tab.id}
          onClick={() => onChange(tab.id)}
          className={`px-4 py-2.5 text-sm font-medium transition-colors relative
            ${current === tab.id
              ? 'text-theme-primary'
              : 'text-theme-tertiary hover:text-theme-secondary'
            }`}
        >
          {tab.label}
          {current === tab.id && (
            <div className="absolute bottom-0 left-2 right-2 h-0.5 bg-[rgb(var(--accent-primary))] rounded-full" />
          )}
        </button>
      ))}
    </div>
  )
}

// ─── Toolbar (Search + Sort + View Toggle) ──────────────────────

interface ToolbarProps {
  view: LibraryView
  searchQuery: string
  onSearchChange: (q: string) => void
  sortField: SortField
  sortDirection: SortDirection
  onSort: (field: SortField) => void
  displayMode: 'grid' | 'list'
  onDisplayModeChange: (mode: 'grid' | 'list') => void
}

const SORT_OPTIONS: Record<LibraryView, { field: SortField; label: string }[]> = {
  artists: [
    { field: 'name', label: 'Name' },
    { field: 'trackCount', label: 'Tracks' },
    { field: 'albumCount', label: 'Albums' },
  ],
  albums: [
    { field: 'title', label: 'Title' },
    { field: 'artist', label: 'Artist' },
    { field: 'year', label: 'Year' },
    { field: 'trackCount', label: 'Tracks' },
  ],
  tracks: [
    { field: 'title', label: 'Title' },
    { field: 'artist', label: 'Artist' },
    { field: 'duration', label: 'Duration' },
    { field: 'format', label: 'Format' },
  ],
  genres: [],
}

function Toolbar({ view, searchQuery, onSearchChange, sortField, sortDirection, onSort, displayMode, onDisplayModeChange }: ToolbarProps) {
  const [sortOpen, setSortOpen] = useState(false)
  const sortRef = useRef<HTMLDivElement>(null)
  const sortOptions = SORT_OPTIONS[view] || []

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (sortRef.current && !sortRef.current.contains(e.target as Node)) setSortOpen(false)
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [])

  return (
    <div className="flex items-center gap-3 py-4">
      {/* Search */}
      <div className="relative flex-1 max-w-sm">
        <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-theme-muted" />
        <input
          type="text"
          value={searchQuery}
          onChange={e => onSearchChange(e.target.value)}
          placeholder={`Search ${view}...`}
          className="w-full pl-9 pr-3 py-2 text-sm bg-surface-raised border border-theme-subtle rounded-lg
            text-theme-primary placeholder:text-theme-muted
            focus:outline-none focus:border-[rgb(var(--accent-primary))] focus:ring-1 focus:ring-[rgb(var(--accent-primary))/0.3]
            transition-colors"
        />
        {searchQuery && (
          <button
            onClick={() => onSearchChange('')}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-theme-muted hover:text-theme-secondary p-0.5"
          >
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.28 7.22a.75.75 0 00-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 101.06 1.06L10 11.06l1.72 1.72a.75.75 0 101.06-1.06L11.06 10l1.72-1.72a.75.75 0 00-1.06-1.06L10 8.94 8.28 7.22z" clipRule="evenodd" />
            </svg>
          </button>
        )}
      </div>

      {/* Sort dropdown */}
      {sortOptions.length > 0 && (
        <div className="relative" ref={sortRef}>
          <button
            onClick={() => setSortOpen(!sortOpen)}
            className="flex items-center gap-1.5 px-3 py-2 text-xs font-medium
              bg-surface-raised border border-theme-subtle rounded-lg
              text-theme-secondary hover:text-theme-primary hover:border-theme-default
              transition-colors"
          >
            <SortIcon className="w-3.5 h-3.5" />
            <span>{sortOptions.find(o => o.field === sortField)?.label || 'Sort'}</span>
            <span className="text-theme-muted">{sortDirection === 'asc' ? '↑' : '↓'}</span>
          </button>

          {sortOpen && (
            <div className="absolute right-0 mt-1 w-40 bg-surface-raised border border-theme-subtle rounded-lg shadow-lg z-20 py-1">
              {sortOptions.map(opt => (
                <button
                  key={opt.field}
                  onClick={() => { onSort(opt.field); setSortOpen(false) }}
                  className={`w-full text-left px-3 py-1.5 text-xs transition-colors flex items-center justify-between
                    ${opt.field === sortField
                      ? 'text-[rgb(var(--accent-primary))] bg-[rgba(var(--accent-primary)/0.08)]'
                      : 'text-theme-secondary hover:bg-accent-subtle'
                    }`}
                >
                  <span>{opt.label}</span>
                  {opt.field === sortField && (
                    <span className="text-[10px]">{sortDirection === 'asc' ? '↑' : '↓'}</span>
                  )}
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Grid / List toggle */}
      {view !== 'tracks' && view !== 'genres' && (
        <div className="flex border border-theme-subtle rounded-lg overflow-hidden">
          <button
            onClick={() => onDisplayModeChange('grid')}
            className={`p-2 transition-colors ${displayMode === 'grid'
              ? 'bg-[rgba(var(--accent-primary)/0.15)] text-[rgb(var(--accent-primary))]'
              : 'bg-surface-raised text-theme-muted hover:text-theme-secondary'}`}
            aria-label="Grid view"
          >
            <GridIcon className="w-4 h-4" />
          </button>
          <button
            onClick={() => onDisplayModeChange('list')}
            className={`p-2 transition-colors border-l border-theme-subtle ${displayMode === 'list'
              ? 'bg-[rgba(var(--accent-primary)/0.15)] text-[rgb(var(--accent-primary))]'
              : 'bg-surface-raised text-theme-muted hover:text-theme-secondary'}`}
            aria-label="List view"
          >
            <ListIcon className="w-4 h-4" />
          </button>
        </div>
      )}
    </div>
  )
}

// ─── Filter Bar ─────────────────────────────────────────────────

function FilterBar() {
  const facets = useLibraryStore((s) => s.facets)
  const facetsLoading = useLibraryStore((s) => s.facetsLoading)
  const activeFilters = useLibraryStore(useShallow((s) => s.activeFilters))
  const addFilter = useLibraryStore((s) => s.addFilter)
  const removeFilter = useLibraryStore((s) => s.removeFilter)
  const clearFilters = useLibraryStore((s) => s.clearFilters)

  useEffect(() => {
    useLibraryStore.getState().fetchFacets()
  }, [])

  if (facetsLoading || !facets) return null

  const activeGenre = activeFilters.find(f => f.field === 'Genre')?.value as string | undefined
  const activeFormat = activeFilters.find(f => f.field === 'AudioFormat')?.value as string | undefined
  const activeBitDepth = activeFilters.find(f => f.field === 'BitDepth')?.value as string | undefined

  return (
    <div className="flex flex-wrap items-center gap-2 pb-2">
      {facets.genres.length > 0 && (
        <FilterChip
          label="Genre"
          value={activeGenre}
          options={facets.genres}
          onChange={(v) => v ? addFilter({ field: 'Genre', operator: 'contains', value: v }) : removeFilter('Genre')}
        />
      )}

      {facets.formats.length > 0 && (
        <FilterChip
          label="Format"
          value={activeFormat}
          options={facets.formats.map(f => f.toUpperCase())}
          onChange={(v) => v ? addFilter({ field: 'AudioFormat', operator: 'equals', value: v.toLowerCase() }) : removeFilter('AudioFormat')}
        />
      )}

      {facets.bitDepths.length > 1 && (
        <FilterChip
          label="Bit Depth"
          value={activeBitDepth}
          options={facets.bitDepths.map(b => `${b}-bit`)}
          onChange={(v) => {
            if (v) {
              addFilter({ field: 'BitDepth', operator: 'equals', value: v.replace('-bit', '') })
            } else {
              removeFilter('BitDepth')
            }
          }}
        />
      )}

      {activeFilters.length > 0 && (
        <button
          onClick={clearFilters}
          className="text-xs text-theme-muted hover:text-theme-secondary transition-colors px-2 py-1"
        >
          Clear
        </button>
      )}
    </div>
  )
}

function FilterChip({ label, value, options, onChange }: {
  label: string
  value?: string
  options: string[]
  onChange: (value: string | null) => void
}) {
  const isActive = !!value
  return (
    <div className="relative">
      <select
        value={value ?? ''}
        onChange={e => onChange(e.target.value || null)}
        className={`appearance-none text-xs font-medium rounded-full px-3 py-1.5 pr-7 cursor-pointer transition-all
          focus:outline-none focus:ring-1 focus:ring-[rgb(var(--accent-primary))/0.3]
          ${isActive
            ? 'bg-[rgba(var(--accent-primary)/0.15)] text-[rgb(var(--accent-primary))] border border-[rgba(var(--accent-primary)/0.3)]'
            : 'bg-surface-raised text-theme-secondary border border-theme-subtle hover:border-theme-default'
          }`}
      >
        <option value="">{label}</option>
        {options.map(opt => (
          <option key={opt} value={opt}>{opt}</option>
        ))}
      </select>
      <svg className="absolute right-2 top-1/2 -translate-y-1/2 w-3 h-3 text-theme-muted pointer-events-none" fill="currentColor" viewBox="0 0 20 20">
        <path fillRule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clipRule="evenodd" />
      </svg>
    </div>
  )
}

// ─── Alphabet Sidebar ───────────────────────────────────────────

const ALPHABET = '#ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('')

function AlphabetBar({ items, nameKey, onJump }: {
  items: { id: number; name?: string; title?: string }[]
  nameKey: 'name' | 'title'
  onJump: (letter: string) => void
}) {
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    items.forEach(item => {
      const name = (item as Record<string, unknown>)[nameKey] as string
      if (name) {
        const first = name[0].toUpperCase()
        letters.add(/[A-Z]/.test(first) ? first : '#')
      }
    })
    return letters
  }, [items, nameKey])

  return (
    <div className="hidden lg:flex flex-col items-center gap-0 fixed right-3 top-1/2 -translate-y-1/2 z-10">
      {ALPHABET.map(letter => (
        <button
          key={letter}
          onClick={() => onJump(letter)}
          disabled={!availableLetters.has(letter)}
          className={`w-5 h-5 text-[10px] font-medium rounded transition-colors
            ${availableLetters.has(letter)
              ? 'text-theme-secondary hover:text-[rgb(var(--accent-primary))] hover:bg-[rgba(var(--accent-primary)/0.1)]'
              : 'text-theme-muted/30 cursor-default'
            }`}
        >
          {letter}
        </button>
      ))}
    </div>
  )
}

// ─── Artist Grid / List ─────────────────────────────────────────

function ArtistGrid({ artists, onSelect }: { artists: Artist[]; onSelect: (a: Artist) => void }) {
  return (
    <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
      {artists.map(artist => (
        <button
          key={artist.id}
          onClick={() => onSelect(artist)}
          className="group text-left bg-surface-raised/60 rounded-xl p-4 border border-transparent
            hover:bg-surface-raised hover:border-theme-subtle hover:shadow-sm
            transition-all duration-150"
          data-letter={artist.name[0]?.toUpperCase() || '#'}
        >
          {/* Artist avatar — circle with initial */}
          <div className="w-full aspect-square rounded-full bg-surface-sunken border border-theme-subtle
            flex items-center justify-center mb-3 mx-auto overflow-hidden
            group-hover:border-[rgba(var(--accent-primary)/0.3)] transition-colors">
            <PersonIcon className="w-1/3 h-1/3 text-theme-muted" />
          </div>
          <h3 className="text-sm font-semibold text-theme-primary truncate text-center">{artist.name}</h3>
          <p className="text-theme-muted text-xs text-center mt-0.5">
            {artist.albumCount} {artist.albumCount === 1 ? 'album' : 'albums'} · {artist.trackCount} tracks
          </p>
        </button>
      ))}
    </div>
  )
}

function ArtistList({ artists, onSelect }: { artists: Artist[]; onSelect: (a: Artist) => void }) {
  return (
    <div className="space-y-0.5">
      {artists.map(artist => (
        <button
          key={artist.id}
          onClick={() => onSelect(artist)}
          className="w-full flex items-center gap-4 px-4 py-3 rounded-lg
            hover:bg-accent-subtle transition-colors text-left group"
          data-letter={artist.name[0]?.toUpperCase() || '#'}
        >
          <div className="w-10 h-10 rounded-full bg-surface-sunken border border-theme-subtle
            flex items-center justify-center flex-shrink-0
            group-hover:border-[rgba(var(--accent-primary)/0.3)] transition-colors">
            <PersonIcon className="w-5 h-5 text-theme-muted" />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-medium text-theme-primary truncate">{artist.name}</h3>
          </div>
          <span className="text-xs text-theme-muted tabular-nums">
            {artist.albumCount} albums · {artist.trackCount} tracks
          </span>
        </button>
      ))}
    </div>
  )
}

// ─── Genre Cards ────────────────────────────────────────────────

function GenreGrid() {
  const facets = useLibraryStore((s) => s.facets)
  const facetsLoading = useLibraryStore((s) => s.facetsLoading)
  const selectGenre = useLibraryStore((s) => s.selectGenre)

  useEffect(() => {
    useLibraryStore.getState().fetchFacets()
  }, [])

  if (facetsLoading) {
    return (
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
        {Array.from({ length: 12 }).map((_, i) => (
          <Skeleton key={i} className="h-16 rounded-xl" />
        ))}
      </div>
    )
  }

  if (!facets || facets.genres.length === 0) {
    return (
      <EmptyState
        icon={<svg className="w-10 h-10 text-theme-muted" fill="currentColor" viewBox="0 0 20 20"><path fillRule="evenodd" d="M17.707 9.293a1 1 0 010 1.414l-7 7a1 1 0 01-1.414 0l-7-7A.997.997 0 012 10V5a3 3 0 013-3h5c.256 0 .512.098.707.293l7 7zM5 6a1 1 0 100-2 1 1 0 000 2z" clipRule="evenodd"/></svg>}
        title="No genres found"
        subtitle="Genre data comes from your library metadata"
      />
    )
  }

  return (
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
      {facets.genres.sort().map(genre => (
        <button
          key={genre}
          onClick={() => selectGenre(genre)}
          className="text-left px-4 py-3 rounded-xl bg-surface-raised/60 border border-transparent
            hover:bg-accent-subtle hover:border-theme-subtle transition-all group"
        >
          <span className="text-sm font-medium text-theme-primary">{genre}</span>
        </button>
      ))}
    </div>
  )
}

// ─── Main Component ─────────────────────────────────────────────

export function LibraryPage() {
  const navigate = useNavigate()
  const { setCurrentTrack, setIsPlaying } = usePlayerStore()
  const { startRadio } = useRadioStore()
  const radioEnabled = isLastfmConfigured()
  const openArtwork = useArtworkViewer((s) => s.open)

  const [searchQuery, setSearchQuery] = useState('')
  const [displayMode, setDisplayMode] = useState<'grid' | 'list'>(() => {
    try { return (localStorage.getItem('akroasis_display_mode') as 'grid' | 'list') || 'grid' } catch { return 'grid' }
  })

  const {
    view, setView,
    artists, albums, tracks,
    isLoading, error,
    totalCount, hasMore,
    activeFilters,
    sortField, sortDirection, setSort,
    selectedArtist, selectedAlbum, selectedGenre,
    fetchArtists, fetchAlbums, fetchTracks,
    selectArtist, selectAlbum, selectGenre,
    goBack, loadMore,
  } = useLibraryStore(useShallow((s) => ({
    view: s.view, setView: s.setView,
    artists: s.artists, albums: s.albums, tracks: s.tracks,
    isLoading: s.isLoading, error: s.error,
    totalCount: s.totalCount, hasMore: s.hasMore,
    activeFilters: s.activeFilters,
    sortField: s.sortField, sortDirection: s.sortDirection, setSort: s.setSort,
    selectedArtist: s.selectedArtist, selectedAlbum: s.selectedAlbum, selectedGenre: s.selectedGenre,
    fetchArtists: s.fetchArtists, fetchAlbums: s.fetchAlbums, fetchTracks: s.fetchTracks,
    selectArtist: s.selectArtist, selectAlbum: s.selectAlbum, selectGenre: s.selectGenre,
    goBack: s.goBack, loadMore: s.loadMore,
  })))

  const suggestedGenres = useListeningProfileStore(useShallow((s) => s.getSuggestedGenres(6)))
  const hasTimeConfidence = useListeningProfileStore((s) => s.hasConfidence('timeOfDay'))

  // Save display mode preference
  useEffect(() => {
    localStorage.setItem('akroasis_display_mode', displayMode)
  }, [displayMode])

  // Initial load
  useEffect(() => {
    if (isLoading) return
    if (view === 'artists' && artists.length === 0 && !selectedArtist) fetchArtists()
    if (view === 'tracks' && tracks.length === 0 && !selectedAlbum && activeFilters.length === 0) fetchTracks()
    if (view === 'albums' && albums.length === 0 && !selectedArtist && activeFilters.length === 0) fetchAlbums()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view])

  // Client-side search filter
  const filteredArtists = useMemo(() => {
    if (!searchQuery) return artists
    const q = searchQuery.toLowerCase()
    return artists.filter(a => a.name.toLowerCase().includes(q))
  }, [artists, searchQuery])

  const filteredAlbums = useMemo(() => {
    if (!searchQuery) return albums
    const q = searchQuery.toLowerCase()
    return albums.filter(a =>
      a.title.toLowerCase().includes(q) ||
      (a.artist && a.artist.toLowerCase().includes(q))
    )
  }, [albums, searchQuery])

  const filteredTracks = useMemo(() => {
    if (!searchQuery) return tracks
    const q = searchQuery.toLowerCase()
    return tracks.filter(t =>
      t.title.toLowerCase().includes(q) ||
      (t.artist && t.artist.toLowerCase().includes(q)) ||
      (t.album && t.album.toLowerCase().includes(q))
    )
  }, [tracks, searchQuery])

  const handleTrackSelect = useCallback((track: Track) => {
    setCurrentTrack(track)
    setIsPlaying(true)
    navigate('/player')
  }, [setCurrentTrack, setIsPlaying, navigate])

  const handleStartRadio = useCallback(async (track: Track, e: React.MouseEvent) => {
    e.stopPropagation()
    await startRadio(track)
    navigate('/queue')
  }, [startRadio, navigate])

  const handleAlphabetJump = useCallback((letter: string) => {
    const el = document.querySelector(`[data-letter="${letter}"]`)
    if (el) el.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }, [])

  // State
  const isDrillDown = !!selectedArtist || !!selectedAlbum
  const showTabs = !isDrillDown
  const showFilters = !isDrillDown && view !== 'genres'
  const showGenres = view === 'genres' && !selectedGenre && !isDrillDown
  const showToolbar = !isDrillDown && !showGenres

  // Header
  let headerTitle = 'Library'
  if (selectedAlbum) headerTitle = selectedAlbum.title
  else if (selectedArtist) headerTitle = selectedArtist.name
  else if (selectedGenre) headerTitle = selectedGenre

  let headerSubtitle = ''
  if (selectedAlbum && selectedArtist) headerSubtitle = `${selectedArtist.name} · ${tracks.length} tracks`
  else if (selectedArtist) headerSubtitle = `${albums.length} albums`
  else if (selectedGenre) headerSubtitle = `${totalCount} tracks`

  return (
    <div className="max-w-7xl mx-auto px-4 py-6">
      {/* Header */}
      <div className="flex items-center gap-3 mb-2">
        {(isDrillDown || selectedGenre) && (
          <button onClick={goBack} className="p-1.5 rounded-lg hover:bg-accent-subtle transition-colors text-theme-secondary">
            <ChevronIcon className="w-5 h-5" />
          </button>
        )}
        <div>
          <h1 className="text-2xl font-serif font-semibold" style={{ color: 'rgb(var(--text-primary))' }}>{headerTitle}</h1>
          {headerSubtitle && (
            <p className="text-theme-muted text-sm">{headerSubtitle}</p>
          )}
        </div>
      </div>

      {/* View tabs */}
      {showTabs && <ViewTabs current={view} onChange={setView} />}

      {/* Toolbar (search + sort + view toggle) */}
      {showToolbar && (
        <Toolbar
          view={view}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          sortField={sortField}
          sortDirection={sortDirection}
          onSort={setSort}
          displayMode={displayMode}
          onDisplayModeChange={setDisplayMode}
        />
      )}

      {/* Filter bar */}
      {showFilters && <FilterBar />}

      {/* Time-aware suggestions */}
      {showTabs && hasTimeConfidence && suggestedGenres.length > 0 && activeFilters.length === 0 && (
        <div className="flex flex-wrap gap-2 mb-4">
          {suggestedGenres.map(genre => (
            <button
              key={genre}
              onClick={() => selectGenre(genre)}
              className="px-3 py-1.5 text-xs rounded-full
                bg-[rgba(var(--accent-primary)/0.08)] text-[rgb(var(--text-secondary))]
                border border-[rgba(var(--accent-primary)/0.12)]
                hover:bg-[rgba(var(--accent-primary)/0.15)] hover:text-[rgb(var(--text-primary))]
                transition-colors"
            >
              {genre}
            </button>
          ))}
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="bg-[rgba(var(--error-bg))] border border-[rgba(var(--error-border))] text-[rgb(var(--error-text))] px-4 py-3 rounded-lg mb-4 text-sm">
          {error}
        </div>
      )}

      {/* Genre view */}
      {showGenres && <GenreGrid />}

      {/* Genre selected — show filtered tracks */}
      {view === 'genres' && selectedGenre && !isDrillDown && (
        <TracksTable
          tracks={filteredTracks}
          isLoading={isLoading}
          onSelect={handleTrackSelect}
          onRadio={radioEnabled ? handleStartRadio : undefined}
        />
      )}

      {/* Artists view */}
      {view === 'artists' && !isDrillDown && (
        isLoading && artists.length === 0 ? (
          <div className={displayMode === 'grid'
            ? 'grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4'
            : 'space-y-0.5'}>
            {Array.from({ length: displayMode === 'grid' ? 18 : 10 }).map((_, i) => <ArtistCardSkeleton key={i} />)}
          </div>
        ) : filteredArtists.length === 0 ? (
          <EmptyState
            icon={<PersonIcon className="w-10 h-10 text-theme-muted" />}
            title={searchQuery ? 'No matching artists' : 'No artists found'}
            subtitle={searchQuery ? `No artists match "${searchQuery}"` : 'Add some music to your library'}
          />
        ) : (
          <>
            {displayMode === 'grid'
              ? <ArtistGrid artists={filteredArtists} onSelect={selectArtist} />
              : <ArtistList artists={filteredArtists} onSelect={selectArtist} />
            }
            <AlphabetBar items={filteredArtists} nameKey="name" onJump={handleAlphabetJump} />
          </>
        )
      )}

      {/* Albums view (drill-down or top-level) */}
      {(view === 'albums' || (isDrillDown && !selectedAlbum)) && !showGenres && (
        isLoading && albums.length === 0 ? (
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
            {Array.from({ length: 10 }).map((_, i) => <AlbumCardSkeleton key={i} />)}
          </div>
        ) : filteredAlbums.length === 0 ? (
          <EmptyState
            icon={<MusicNoteIcon className="w-10 h-10 text-theme-muted" />}
            title={searchQuery ? 'No matching albums' : 'No albums found'}
            subtitle={selectedArtist ? `No albums for ${selectedArtist.name}` : searchQuery ? `No albums match "${searchQuery}"` : 'No albums in library'}
          />
        ) : (
          <div className={displayMode === 'grid'
            ? 'grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4'
            : 'space-y-0.5'
          }>
            {filteredAlbums.map(album => displayMode === 'grid' ? (
              <div
                key={`${album.id}-${album.title}`}
                onClick={() => selectAlbum(album)}
                className="group cursor-pointer bg-surface-raised/60 rounded-xl overflow-hidden border border-transparent
                  hover:bg-surface-raised hover:border-theme-subtle hover:shadow-sm
                  transition-all duration-150"
                role="button"
                tabIndex={0}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); selectAlbum(album) }}}
                data-letter={album.title[0]?.toUpperCase() || '#'}
              >
                <div className="w-full aspect-square bg-surface-sunken overflow-hidden">
                  {album.coverArtUrl ? (
                    <img
                      src={authenticateUrl(album.coverArtUrl)}
                      alt={album.title}
                      className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                      loading="lazy"
                      onClick={(e) => { e.stopPropagation(); openArtwork(authenticateUrl(album.coverArtUrl)!) }}
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center">
                      <MusicNoteIcon className="w-12 h-12 text-theme-muted" />
                    </div>
                  )}
                </div>
                <div className="p-3">
                  <h3 className="text-sm font-semibold text-theme-primary truncate">{album.title}</h3>
                  <p className="text-theme-tertiary text-xs mt-0.5 truncate">{album.artist}</p>
                  <p className="text-theme-muted text-[11px] mt-1 tabular-nums">
                    {album.year && `${album.year} · `}
                    {album.trackCount ? `${album.trackCount} tracks` : ''}
                    {album.duration ? ` · ${Math.floor(album.duration / 60)}m` : ''}
                  </p>
                </div>
              </div>
            ) : (
              <button
                key={`${album.id}-${album.title}`}
                onClick={() => selectAlbum(album)}
                className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg
                  hover:bg-accent-subtle transition-colors text-left group"
                data-letter={album.title[0]?.toUpperCase() || '#'}
              >
                <div className="w-12 h-12 rounded-lg bg-surface-sunken overflow-hidden flex-shrink-0 border border-theme-subtle">
                  {album.coverArtUrl ? (
                    <img src={authenticateUrl(album.coverArtUrl)} alt="" className="w-full h-full object-cover" loading="lazy" />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center">
                      <MusicNoteIcon className="w-5 h-5 text-theme-muted" />
                    </div>
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <h3 className="text-sm font-medium text-theme-primary truncate">{album.title}</h3>
                  <p className="text-xs text-theme-tertiary truncate">{album.artist}</p>
                </div>
                <span className="text-xs text-theme-muted tabular-nums flex-shrink-0">
                  {album.year && `${album.year} · `}{album.trackCount || 0} tracks
                </span>
              </button>
            ))}
          </div>
        )
      )}

      {/* Tracks view (drill-down or top-level) */}
      {(view === 'tracks' || (isDrillDown && selectedAlbum)) && !showGenres && (
        <TracksTable
          tracks={filteredTracks}
          isLoading={isLoading}
          onSelect={handleTrackSelect}
          onRadio={radioEnabled ? handleStartRadio : undefined}
        />
      )}

      {/* Load more */}
      {hasMore && !isLoading && (
        <div className="flex justify-center mt-8">
          <Button onClick={loadMore} variant="secondary" size="sm">
            Load more
          </Button>
        </div>
      )}

      {/* Loading spinner */}
      {isLoading && (view === 'artists' ? artists.length > 0 : tracks.length > 0) && (
        <div className="flex justify-center mt-6">
          <div className="w-5 h-5 border-2 border-theme-strong border-t-accent rounded-full animate-spin" />
        </div>
      )}
    </div>
  )
}

// ─── Tracks Table ───────────────────────────────────────────────

function TracksTable({ tracks, isLoading, onSelect, onRadio }: {
  tracks: Track[]
  isLoading: boolean
  onSelect: (track: Track) => void
  onRadio?: (track: Track, e: React.MouseEvent) => void
}) {
  if (isLoading && tracks.length === 0) {
    return (
      <div className="space-y-1">
        {Array.from({ length: 10 }).map((_, i) => <TrackRowSkeleton key={i} />)}
      </div>
    )
  }

  if (tracks.length === 0) {
    return (
      <EmptyState
        icon={<MusicNoteIcon className="w-10 h-10 text-theme-muted" />}
        title="No tracks"
        subtitle="No tracks match the current filters"
      />
    )
  }

  return (
    <div className="space-y-0.5">
      {tracks.map((track, index) => (
        <div
          key={track.id}
          onClick={() => onSelect(track)}
          className="flex items-center gap-4 px-4 py-2.5 rounded-lg cursor-pointer hover:bg-accent-subtle transition-colors group"
          role="button"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect(track) }}}
        >
          <span className="w-6 text-right text-sm text-theme-muted tabular-nums group-hover:hidden">
            {index + 1}
          </span>
          <svg className="w-6 h-6 text-theme-tertiary hidden group-hover:block flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
            <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
          </svg>

          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-medium text-theme-primary truncate">{track.title}</h3>
            <p className="text-xs text-theme-tertiary truncate">{track.artist}{track.album ? ` · ${track.album}` : ''}</p>
          </div>

          <QualityDot tier={getSourceTier(track)} className="flex-shrink-0 hidden sm:inline-block" />

          <span className="text-[11px] text-theme-muted tabular-nums hidden md:inline">
            {track.format?.toUpperCase()}{track.sampleRate ? ` · ${(track.sampleRate / 1000).toFixed(1)}kHz` : ''}{track.bitDepth ? `/${track.bitDepth}` : ''}
          </span>

          <HeartButton trackId={track.id} />

          {onRadio && (
            <button
              onClick={(e) => onRadio(track, e)}
              className="opacity-0 group-hover:opacity-100 text-theme-tertiary hover:text-theme-secondary transition-all"
              title="Start Radio"
              aria-label={`Start radio from ${track.title}`}
            >
              <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M9.383 3.076A1 1 0 0110 4v12a1 1 0 01-1.707.707L4.586 13H2a1 1 0 01-1-1V8a1 1 0 011-1h2.586l3.707-3.707a1 1 0 011.09-.217zM14.657 2.929a1 1 0 011.414 0A9.972 9.972 0 0119 10a9.972 9.972 0 01-2.929 7.071 1 1 0 01-1.414-1.414A7.971 7.971 0 0017 10c0-2.21-.894-4.208-2.343-5.657a1 1 0 010-1.414zm-2.829 2.828a1 1 0 011.415 0A5.983 5.983 0 0115 10a5.983 5.983 0 01-1.757 4.243 1 1 0 01-1.415-1.415A3.984 3.984 0 0013 10a3.984 3.984 0 00-1.172-2.828 1 1 0 010-1.415z" clipRule="evenodd"/>
              </svg>
            </button>
          )}

          <span className="text-xs text-theme-tertiary tabular-nums w-12 text-right">
            {Math.floor(track.duration / 60)}:{String(track.duration % 60).padStart(2, '0')}
          </span>
        </div>
      ))}
    </div>
  )
}
