import { useEffect, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { useShallow } from 'zustand/react/shallow'
import { isLastfmConfigured } from '../api/lastfm'
import type { Track, FilterCondition } from '../types'
import { authenticateUrl } from '../api/client'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import { usePlayerStore } from '../stores/playerStore'
import { useRadioStore } from '../stores/radioStore'
import { useLibraryStore, type LibraryView } from '../stores/libraryStore'
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
    <div className="bg-surface-raised border border-theme-subtle rounded-xl p-5">
      <Skeleton className="h-6 w-3/4 mb-3" />
      <Skeleton className="h-4 w-1/2" />
    </div>
  )
}

function AlbumCardSkeleton() {
  return (
    <div className="bg-surface-raised border border-theme-subtle rounded-xl overflow-hidden">
      <Skeleton className="w-full aspect-square rounded-none" />
      <div className="p-4">
        <Skeleton className="h-5 w-3/4 mb-2" />
        <Skeleton className="h-4 w-1/2 mb-2" />
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

// ─── View Tab Bar ───────────────────────────────────────────────

const VIEW_TABS: { id: LibraryView; label: string }[] = [
  { id: 'artists', label: 'Artists' },
  { id: 'albums', label: 'Albums' },
  { id: 'tracks', label: 'Tracks' },
  { id: 'genres', label: 'Genres' },
]

function ViewTabs({ current, onChange }: { current: LibraryView; onChange: (v: LibraryView) => void }) {
  return (
    <div className="flex gap-1 border-b border-theme-subtle mb-6">
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

  const activeGenre = activeFilters.find(f => f.field === 'genres')?.value as string | undefined
  const activeFormat = activeFilters.find(f => f.field === 'audioFormat')?.value as string | undefined
  const activeBitDepth = activeFilters.find(f => f.field === 'bitDepth')?.value as string | undefined

  return (
    <div className="mb-6">
      {/* Filter dropdowns */}
      <div className="flex flex-wrap items-center gap-3">
        <span className="text-xs text-theme-tertiary uppercase tracking-wider font-medium">Filters</span>

        {/* Genre */}
        {facets.genres.length > 0 && (
          <FilterDropdown
            label="Genre"
            value={activeGenre}
            options={facets.genres}
            onChange={(v) => v ? addFilter({ field: 'genres', operator: 'contains', value: v }) : removeFilter('genres')}
          />
        )}

        {/* Format */}
        {facets.formats.length > 0 && (
          <FilterDropdown
            label="Format"
            value={activeFormat}
            options={facets.formats.map(f => f.toUpperCase())}
            onChange={(v) => v ? addFilter({ field: 'audioFormat', operator: 'equals', value: v.toLowerCase() }) : removeFilter('audioFormat')}
          />
        )}

        {/* Bit Depth */}
        {facets.bitDepths.length > 1 && (
          <FilterDropdown
            label="Bit Depth"
            value={activeBitDepth}
            options={facets.bitDepths.map(b => `${b}-bit`)}
            onChange={(v) => {
              if (v) {
                const num = v.replace('-bit', '')
                addFilter({ field: 'bitDepth', operator: 'equals', value: num })
              } else {
                removeFilter('bitDepth')
              }
            }}
          />
        )}

        {activeFilters.length > 0 && (
          <button
            onClick={clearFilters}
            className="text-xs text-theme-tertiary hover:text-theme-secondary transition-colors ml-1"
          >
            Clear all
          </button>
        )}
      </div>

      {/* Active filter pills */}
      {activeFilters.length > 0 && (
        <div className="flex flex-wrap gap-2 mt-3">
          {activeFilters.map(f => (
            <FilterPill key={f.field} condition={f} onRemove={() => removeFilter(f.field)} />
          ))}
        </div>
      )}
    </div>
  )
}

function FilterDropdown({ label, value, options, onChange }: {
  label: string
  value?: string
  options: string[]
  onChange: (value: string | null) => void
}) {
  return (
    <select
      value={value ?? ''}
      onChange={e => onChange(e.target.value || null)}
      className="bg-surface-raised border border-theme-subtle text-theme-primary text-xs rounded-lg px-3 py-1.5
        focus:outline-none focus:border-theme-strong cursor-pointer"
    >
      <option value="">{label}: All</option>
      {options.map(opt => (
        <option key={opt} value={opt}>{opt}</option>
      ))}
    </select>
  )
}

function FilterPill({ condition, onRemove }: { condition: FilterCondition; onRemove: () => void }) {
  const label = `${condition.field === 'genres' ? 'Genre' : condition.field === 'audioFormat' ? 'Format' : condition.field}: ${condition.value}`
  return (
    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs rounded-full
      bg-[rgba(var(--accent-primary)/0.15)] text-[rgb(var(--accent-primary))] border border-[rgba(var(--accent-primary)/0.3)]">
      {label}
      <button onClick={onRemove} className="hover:text-theme-primary transition-colors" aria-label={`Remove ${label} filter`}>
        ×
      </button>
    </span>
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
          <Skeleton key={i} className="h-20 rounded-xl" />
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
    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
      {facets.genres.sort().map(genre => (
        <button
          key={genre}
          onClick={() => selectGenre(genre)}
          className="text-left p-4 rounded-xl bg-surface-raised/80 border border-theme-subtle
            hover:bg-accent-subtle hover:border-theme-default transition-all group"
        >
          <span className="text-sm font-medium text-theme-primary group-hover:text-theme-primary">
            {genre}
          </span>
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

  const {
    view, setView,
    artists, albums, tracks,
    isLoading, error,
    totalCount, hasMore,
    activeFilters,
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
    selectedArtist: s.selectedArtist, selectedAlbum: s.selectedAlbum, selectedGenre: s.selectedGenre,
    fetchArtists: s.fetchArtists, fetchAlbums: s.fetchAlbums, fetchTracks: s.fetchTracks,
    selectArtist: s.selectArtist, selectAlbum: s.selectAlbum, selectGenre: s.selectGenre,
    goBack: s.goBack, loadMore: s.loadMore,
  })))

  // Stable selectors — getSuggestedGenres returns new array each call,
  // which without shallow comparison causes infinite re-render loops
  const suggestedGenres = useListeningProfileStore(useShallow((s) => s.getSuggestedGenres(6)))
  const hasTimeConfidence = useListeningProfileStore((s) => s.hasConfidence('timeOfDay'))

  // Initial load — guard with isLoading to prevent double-fetch during state transitions
  useEffect(() => {
    if (isLoading) return
    if (view === 'artists' && artists.length === 0 && !selectedArtist) fetchArtists()
    if (view === 'tracks' && tracks.length === 0 && !selectedAlbum && activeFilters.length === 0) fetchTracks()
    if (view === 'albums' && albums.length === 0 && !selectedArtist && activeFilters.length === 0) fetchAlbums()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view])

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

  // Determine if we're in drill-down mode
  const isDrillDown = !!selectedArtist || !!selectedAlbum
  const showTabs = !isDrillDown
  const showFilters = !isDrillDown && view !== 'genres'
  const showGenres = view === 'genres' && !selectedGenre && !isDrillDown

  // Header title
  let headerTitle = 'Library'
  if (selectedAlbum) headerTitle = selectedAlbum.title
  else if (selectedArtist) headerTitle = selectedArtist.name
  else if (selectedGenre) headerTitle = selectedGenre

  let headerSubtitle = ''
  if (selectedAlbum && selectedArtist) headerSubtitle = `${selectedArtist.name} · ${tracks.length} tracks`
  else if (selectedArtist) headerSubtitle = `${albums.length} albums`
  else if (selectedGenre) headerSubtitle = `${totalCount} tracks`
  else if (view === 'tracks' && !isDrillDown) headerSubtitle = `${totalCount} tracks`
  else if (view === 'albums' && !isDrillDown) headerSubtitle = `${albums.length} albums`

  return (
    <div className="max-w-7xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-4">
          {(isDrillDown || selectedGenre) && (
            <Button onClick={goBack} variant="secondary" size="sm">
              <svg className="w-4 h-4 mr-1" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
              </svg>
              Back
            </Button>
          )}
          <div>
            <h1 className="text-3xl font-serif font-semibold" style={{ color: 'rgb(var(--text-primary))' }}>{headerTitle}</h1>
            {headerSubtitle && (
              <p className="text-theme-tertiary text-sm mt-0.5">{headerSubtitle}</p>
            )}
          </div>
        </div>
      </div>

      {/* View tabs */}
      {showTabs && <ViewTabs current={view} onChange={setView} />}

      {/* Filter bar */}
      {showFilters && <FilterBar />}

      {/* Time-aware suggestions — only shown with sufficient data */}
      {showTabs && hasTimeConfidence && suggestedGenres.length > 0 && activeFilters.length === 0 && (
        <div className="flex flex-wrap gap-2 mb-6">
          {suggestedGenres.map(genre => (
            <button
              key={genre}
              onClick={() => selectGenre(genre)}
              className="px-3 py-1.5 text-xs rounded-full
                bg-[rgba(var(--accent-primary)/0.1)] text-[rgb(var(--text-secondary))]
                border border-[rgba(var(--accent-primary)/0.15)]
                hover:bg-[rgba(var(--accent-primary)/0.2)] hover:text-[rgb(var(--text-primary))]
                transition-colors"
            >
              {genre}
            </button>
          ))}
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="bg-[rgba(var(--error-bg))] border border-[rgba(var(--error-border))] text-[rgb(var(--error-text))] px-4 py-3 rounded-lg mb-6 text-sm">
          {error}
        </div>
      )}

      {/* Genre view */}
      {showGenres && <GenreGrid />}

      {/* Genre selected — show filtered tracks */}
      {view === 'genres' && selectedGenre && !isDrillDown && (
        <TracksTable
          tracks={tracks}
          isLoading={isLoading}
          onSelect={handleTrackSelect}
          onRadio={radioEnabled ? handleStartRadio : undefined}
        />
      )}

      {/* Artists view */}
      {view === 'artists' && !isDrillDown && (
        isLoading && artists.length === 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {Array.from({ length: 9 }).map((_, i) => <ArtistCardSkeleton key={i} />)}
          </div>
        ) : artists.length === 0 ? (
          <EmptyState
            icon={<svg className="w-10 h-10 text-theme-muted" fill="currentColor" viewBox="0 0 20 20"><path d="M13 6a3 3 0 11-6 0 3 3 0 016 0zM18 8a2 2 0 11-4 0 2 2 0 014 0zM14 15a4 4 0 00-8 0v3h8v-3zM6 8a2 2 0 11-4 0 2 2 0 014 0zM16 18v-3a5.972 5.972 0 00-.75-2.906A3.005 3.005 0 0119 15v3h-3zM4.75 12.094A5.973 5.973 0 004 15v3H1v-3a3 3 0 013.75-2.906z"/></svg>}
            title="No artists found"
            subtitle="Add some music to your Mouseion library"
          />
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {artists.map(artist => (
              <Card
                key={artist.id}
                onClick={() => selectArtist(artist)}
                className="cursor-pointer hover:bg-surface-sunken hover:border-theme-default hover:scale-[1.01] transition-all duration-150"
              >
                <div className="p-1">
                  <h3 className="text-lg font-semibold text-theme-primary mb-1">{artist.name}</h3>
                  <p className="text-theme-tertiary text-sm">
                    {artist.albumCount} {artist.albumCount === 1 ? 'album' : 'albums'} · {artist.trackCount} tracks
                  </p>
                </div>
              </Card>
            ))}
          </div>
        )
      )}

      {/* Albums view (drill-down or top-level) */}
      {(view === 'albums' || (isDrillDown && !selectedAlbum)) && !showGenres && (
        isLoading && albums.length === 0 ? (
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
            {Array.from({ length: 8 }).map((_, i) => <AlbumCardSkeleton key={i} />)}
          </div>
        ) : albums.length === 0 ? (
          <EmptyState
            icon={<svg className="w-10 h-10 text-theme-muted" fill="currentColor" viewBox="0 0 20 20"><path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/></svg>}
            title="No albums found"
            subtitle={selectedArtist ? `No albums for ${selectedArtist.name}` : 'No albums in library'}
          />
        ) : (
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
            {albums.map(album => (
              <div
                key={`${album.id}-${album.title}`}
                onClick={() => selectAlbum(album)}
                className="group cursor-pointer bg-surface-raised/80 rounded-xl overflow-hidden border border-theme-subtle hover:bg-accent-subtle hover:border-theme-default hover:scale-[1.02] transition-all duration-150"
                role="button"
                tabIndex={0}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); selectAlbum(album) }}}
              >
                <div className="w-full aspect-square bg-surface-sunken overflow-hidden">
                  {album.coverArtUrl ? (
                    <img
                      src={authenticateUrl(album.coverArtUrl)}
                      alt={album.title}
                      className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
                      onClick={(e) => { e.stopPropagation(); openArtwork(authenticateUrl(album.coverArtUrl)!) }}
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center">
                      <svg className="w-12 h-12 text-theme-muted" fill="currentColor" viewBox="0 0 20 20">
                        <path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/>
                      </svg>
                    </div>
                  )}
                </div>
                <div className="p-3">
                  <h3 className="text-sm font-semibold text-theme-primary truncate">{album.title}</h3>
                  <p className="text-theme-tertiary text-xs mt-0.5 truncate">{album.artist}</p>
                  <p className="text-theme-muted text-xs mt-1">
                    {album.year && `${album.year} · `}
                    {album.trackCount ? `${album.trackCount} tracks` : 'Album'}
                    {album.duration ? ` · ${Math.floor(album.duration / 60)}min` : ''}
                  </p>
                </div>
              </div>
            ))}
          </div>
        )
      )}

      {/* Tracks view (drill-down or top-level) */}
      {(view === 'tracks' || (isDrillDown && selectedAlbum)) && !showGenres && (
        <TracksTable
          tracks={tracks}
          isLoading={isLoading}
          onSelect={handleTrackSelect}
          onRadio={radioEnabled ? handleStartRadio : undefined}
        />
      )}

      {/* Load more */}
      {hasMore && !isLoading && (
        <div className="flex justify-center mt-8">
          <Button onClick={loadMore} variant="secondary">
            Load more
          </Button>
        </div>
      )}

      {/* Loading more indicator */}
      {isLoading && (view === 'artists' ? artists.length > 0 : tracks.length > 0) && (
        <div className="flex justify-center mt-6">
          <div className="w-6 h-6 border-2 border-theme-strong border-t-accent rounded-full animate-spin" />
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
        icon={<svg className="w-10 h-10 text-theme-muted" fill="currentColor" viewBox="0 0 20 20"><path d="M18 3a1 1 0 00-1.196-.98l-10 2A1 1 0 006 5v9.114A4.369 4.369 0 005 14c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V7.82l8-1.6v5.894A4.37 4.37 0 0015 12c-1.657 0-3 .895-3 2s1.343 2 3 2 3-.895 3-2V3z"/></svg>}
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
          className="flex items-center gap-4 px-4 py-3 rounded-lg cursor-pointer hover:bg-accent-subtle transition-colors group"
          role="button"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect(track) }}}
        >
          {/* Track number / play icon */}
          <span className="w-6 text-right text-sm text-theme-muted tabular-nums group-hover:hidden">
            {index + 1}
          </span>
          <svg className="w-6 h-6 text-theme-tertiary hidden group-hover:block flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
            <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd"/>
          </svg>

          {/* Track info */}
          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-medium text-theme-primary truncate">{track.title}</h3>
            <p className="text-xs text-theme-tertiary truncate">{track.artist}</p>
          </div>

          {/* Quality */}
          <QualityDot tier={getSourceTier(track)} className="flex-shrink-0 hidden sm:inline-block" />

          <span className="text-xs text-theme-muted tabular-nums hidden sm:inline">
            {track.format.toUpperCase()} · {(track.sampleRate / 1000).toFixed(1)}kHz/{track.bitDepth}bit
          </span>

          {/* Favorite */}
          <HeartButton trackId={track.id} />

          {/* Radio */}
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

          {/* Duration */}
          <span className="text-xs text-theme-tertiary tabular-nums w-12 text-right">
            {Math.floor(track.duration / 60)}:{String(track.duration % 60).padStart(2, '0')}
          </span>
        </div>
      ))}
    </div>
  )
}
