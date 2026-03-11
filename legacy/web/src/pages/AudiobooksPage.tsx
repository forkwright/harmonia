import { useEffect, useState, useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAudiobookStore } from '../stores/audiobookStore'
import { useContinueStore } from '../stores/continueStore'
import { Card } from '../components/Card'
import { apiClient, authenticateUrl } from '../api/client'
import type { Author, Audiobook } from '../types'

type SortKey = 'title' | 'author' | 'year' | 'duration'
type SortDir = 'asc' | 'desc'
type DisplayMode = 'grid' | 'list'

function formatDuration(minutes?: number): string {
  if (!minutes) return ''
  const hours = Math.floor(minutes / 60)
  const mins = minutes % 60
  if (hours === 0) return `${mins}m`
  return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`
}

function getAuthorName(book: Audiobook, authorMap: Map<number, Author>): string {
  if (book.authorId) {
    const author = authorMap.get(book.authorId)
    if (author) return author.name
  }
  return ''
}

function ContinueListeningSection() {
  const { items: continueItems, fetchItems } = useContinueStore()
  const navigate = useNavigate()

  useEffect(() => {
    fetchItems(10)
  }, [fetchItems])

  if (continueItems.length === 0) return null

  return (
    <div className="mb-8">
      <h2 className="text-xl font-bold text-theme-primary mb-4">Continue Listening</h2>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {continueItems.map((item) => (
          <button
            key={item.mediaItemId}
            onClick={() => navigate(`/audiobooks/play/${item.mediaItemId}`)}
            className="text-left p-4 bg-accent-subtle rounded-xl hover:bg-surface-sunken hover:scale-[1.01] transition-all duration-150 border border-theme-subtle"
          >
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 flex-shrink-0 bg-surface-sunken rounded overflow-hidden">
                <img
                  src={authenticateUrl(apiClient.getAudiobookCoverUrl(item.mediaItemId, 96))}
                  alt={item.title}
                  className="w-full h-full object-cover"
                  onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
                />
              </div>
              <div className="min-w-0 flex-1">
                <p className="font-medium text-theme-primary truncate">{item.title}</p>
                <div className="w-full bg-surface-sunken rounded-full h-1.5 mt-2">
                  <div
                    className="bg-accent h-1.5 rounded-full transition-all"
                    style={{ width: `${Math.min(item.percentComplete, 100)}%` }}
                  />
                </div>
                <p className="text-xs text-theme-tertiary mt-1">{Math.round(item.percentComplete)}% complete</p>
              </div>
            </div>
          </button>
        ))}
      </div>
    </div>
  )
}

function AudiobookGridCard({ audiobook, authorName, onClick }: { audiobook: Audiobook; authorName: string; onClick: () => void }) {
  const coverUrl = authenticateUrl(apiClient.getAudiobookCoverUrl(audiobook.id, 200))

  return (
    <button
      onClick={onClick}
      className="text-left bg-accent-subtle rounded-xl hover:bg-surface-sunken hover:scale-[1.01] transition-all duration-150 border border-theme-subtle overflow-hidden"
    >
      <div className="aspect-square bg-surface-sunken overflow-hidden">
        <img
          src={coverUrl}
          alt={audiobook.title}
          className="w-full h-full object-cover"
          onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
        />
      </div>
      <div className="p-3">
        <h3 className="text-sm font-semibold text-theme-primary truncate">{audiobook.title}</h3>
        {authorName && <p className="text-xs text-theme-tertiary mt-0.5 truncate">{authorName}</p>}
        <div className="flex items-center gap-2 mt-1 text-xs text-theme-muted">
          {audiobook.year > 0 && <span>{audiobook.year}</span>}
          {audiobook.metadata.durationMinutes && (
            <span>{formatDuration(audiobook.metadata.durationMinutes)}</span>
          )}
        </div>
      </div>
    </button>
  )
}

function AudiobookListCard({ audiobook, authorName, onClick }: { audiobook: Audiobook; authorName: string; onClick: () => void }) {
  const coverUrl = authenticateUrl(apiClient.getAudiobookCoverUrl(audiobook.id, 200))

  return (
    <button
      onClick={onClick}
      className="w-full text-left bg-accent-subtle rounded-xl hover:bg-surface-sunken hover:scale-[1.01] transition-all duration-150 border border-theme-subtle overflow-hidden"
    >
      <div className="flex gap-4 p-4">
        <div className="w-20 h-20 flex-shrink-0 bg-surface-sunken rounded overflow-hidden">
          <img
            src={coverUrl}
            alt={audiobook.title}
            className="w-full h-full object-cover"
            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
          />
        </div>
        <div className="min-w-0 flex-1">
          <h3 className="text-lg font-semibold text-theme-primary truncate">{audiobook.title}</h3>
          {authorName && <p className="text-sm text-theme-tertiary mt-0.5 truncate">{authorName}</p>}
          {audiobook.metadata.narrator && (
            <p className="text-xs text-theme-tertiary mt-0.5">
              Narrated by {audiobook.metadata.narrators.length > 1
                ? audiobook.metadata.narrators.join(', ')
                : audiobook.metadata.narrator}
            </p>
          )}
          <div className="flex items-center gap-3 mt-1 text-xs text-theme-muted">
            {audiobook.year > 0 && <span>{audiobook.year}</span>}
            {audiobook.metadata.durationMinutes && (
              <span>{formatDuration(audiobook.metadata.durationMinutes)}</span>
            )}
            {audiobook.metadata.isAbridged && (
              <span className="px-1.5 py-0.5 bg-surface-sunken rounded text-theme-secondary">Abridged</span>
            )}
            {audiobook.metadata.seriesPosition != null && (
              <span>Book {audiobook.metadata.seriesPosition}</span>
            )}
          </div>
          {audiobook.metadata.genres.length > 0 && (
            <div className="flex gap-1.5 mt-2">
              {audiobook.metadata.genres.slice(0, 3).map((genre) => (
                <span key={genre} className="text-xs px-2 py-0.5 bg-accent-subtle rounded text-theme-secondary">
                  {genre}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>
    </button>
  )
}

export function AudiobooksPage() {
  const navigate = useNavigate()
  const { authors, audiobooks, loading, error, loadAuthors, loadAudiobooks } = useAudiobookStore()

  const [searchQuery, setSearchQuery] = useState('')
  const [sortKey, setSortKey] = useState<SortKey>('title')
  const [sortDir, setSortDir] = useState<SortDir>('asc')
  const [displayMode, setDisplayMode] = useState<DisplayMode>('list')
  const [authorFilter, setAuthorFilter] = useState<string | null>(null)

  useEffect(() => {
    loadAuthors()
    loadAudiobooks()
  }, [loadAuthors, loadAudiobooks])

  // Author lookup map
  const authorMap = useMemo(() => {
    const map = new Map<number, Author>()
    authors.forEach((a) => map.set(a.id, a))
    return map
  }, [authors])

  // Derive unique author names
  const authorNames = useMemo(() => {
    const names = new Set<string>()
    audiobooks.forEach((b) => {
      const name = getAuthorName(b, authorMap)
      if (name) names.add(name)
    })
    return Array.from(names).sort((a, b) => a.localeCompare(b))
  }, [audiobooks, authorMap])

  // Filter + sort
  const filtered = useMemo(() => {
    let list = audiobooks.map((b) => ({ book: b, authorName: getAuthorName(b, authorMap) }))

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase()
      list = list.filter(
        ({ book, authorName }) =>
          book.title.toLowerCase().includes(q) ||
          authorName.toLowerCase().includes(q) ||
          book.metadata.narrator?.toLowerCase().includes(q)
      )
    }

    if (authorFilter) {
      list = list.filter(({ authorName }) => authorName === authorFilter)
    }

    const dir = sortDir === 'asc' ? 1 : -1
    list.sort((a, b) => {
      switch (sortKey) {
        case 'title':
          return dir * a.book.title.localeCompare(b.book.title)
        case 'author':
          return dir * a.authorName.localeCompare(b.authorName)
        case 'year':
          return dir * ((a.book.year || 0) - (b.book.year || 0))
        case 'duration':
          return dir * ((a.book.metadata.durationMinutes || 0) - (b.book.metadata.durationMinutes || 0))
        default:
          return 0
      }
    })

    return list
  }, [audiobooks, authorMap, searchQuery, authorFilter, sortKey, sortDir])

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'))
    } else {
      setSortKey(key)
      setSortDir('asc')
    }
  }

  const handleBookClick = (audiobook: Audiobook) => {
    navigate(`/audiobooks/play/${audiobook.id}`)
  }

  return (
    <div className="max-w-7xl mx-auto p-4">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-serif font-semibold" style={{ color: 'rgb(var(--text-primary))' }}>
          Audiobooks
        </h1>
        <span className="text-sm text-theme-tertiary">{filtered.length} books</span>
      </div>

      {error && (
        <Card>
          <p className="text-red-400">{error}</p>
        </Card>
      )}

      {!loading && <ContinueListeningSection />}

      {/* Toolbar: Search + Sort + Display */}
      <div className="flex flex-col sm:flex-row gap-3 mb-6">
        <div className="flex-1">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search title, author, narrator..."
            className="w-full bg-surface-raised text-theme-primary rounded-lg px-3 py-2 text-sm placeholder-theme-muted focus:outline-none focus:ring-1 focus:ring-accent"
          />
        </div>
        <div className="flex gap-2">
          {(['title', 'author', 'year', 'duration'] as SortKey[]).map((key) => (
            <button
              key={key}
              onClick={() => handleSort(key)}
              className={`px-3 py-1.5 text-xs rounded-lg transition-colors ${
                sortKey === key
                  ? 'bg-accent text-white'
                  : 'bg-surface-raised text-theme-tertiary hover:text-theme-secondary'
              }`}
            >
              {key.charAt(0).toUpperCase() + key.slice(1)}
              {sortKey === key && (sortDir === 'asc' ? ' ↑' : ' ↓')}
            </button>
          ))}
          <button
            onClick={() => setDisplayMode((m) => (m === 'grid' ? 'list' : 'grid'))}
            className="px-3 py-1.5 text-xs rounded-lg bg-surface-raised text-theme-tertiary hover:text-theme-secondary transition-colors"
            title={displayMode === 'grid' ? 'Switch to list' : 'Switch to grid'}
          >
            {displayMode === 'grid' ? '☰' : '▦'}
          </button>
        </div>
      </div>

      {/* Author filter chips */}
      {authorNames.length > 1 && (
        <div className="flex flex-wrap gap-2 mb-6">
          <button
            onClick={() => setAuthorFilter(null)}
            className={`px-3 py-1 text-xs rounded-full transition-colors ${
              !authorFilter ? 'bg-accent text-white' : 'bg-surface-raised text-theme-tertiary hover:text-theme-secondary'
            }`}
          >
            All
          </button>
          {authorNames.map((name) => (
            <button
              key={name}
              onClick={() => setAuthorFilter(authorFilter === name ? null : name)}
              className={`px-3 py-1 text-xs rounded-full transition-colors ${
                authorFilter === name ? 'bg-accent text-white' : 'bg-surface-raised text-theme-tertiary hover:text-theme-secondary'
              }`}
            >
              {name}
            </button>
          ))}
        </div>
      )}

      {loading && (
        <div className="text-center py-12 text-theme-tertiary">Loading...</div>
      )}

      {!loading && filtered.length === 0 && (
        <div className="text-center py-12">
          <p className="text-theme-tertiary">
            {searchQuery.trim() || authorFilter ? 'No books match your search' : 'No audiobooks in library'}
          </p>
        </div>
      )}

      {!loading && filtered.length > 0 && displayMode === 'grid' && (
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
          {filtered.map(({ book, authorName }) => (
            <AudiobookGridCard key={book.id} audiobook={book} authorName={authorName} onClick={() => handleBookClick(book)} />
          ))}
        </div>
      )}

      {!loading && filtered.length > 0 && displayMode === 'list' && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {filtered.map(({ book, authorName }) => (
            <AudiobookListCard key={book.id} audiobook={book} authorName={authorName} onClick={() => handleBookClick(book)} />
          ))}
        </div>
      )}
    </div>
  )
}
