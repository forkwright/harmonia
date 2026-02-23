import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAudiobookStore } from '../stores/audiobookStore'
import { useContinueStore } from '../stores/continueStore'
import { Card } from '../components/Card'
import { Button } from '../components/Button'
import { apiClient, authenticateUrl } from '../api/client'
import type { Author, Audiobook } from '../types'

function formatDuration(minutes?: number): string {
  if (!minutes) return ''
  const hours = Math.floor(minutes / 60)
  const mins = minutes % 60
  if (hours === 0) return `${mins}m`
  return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`
}

function AuthorCard({ author, onClick }: { author: Author; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="w-full text-left p-4 bg-accent-subtle rounded-xl hover:bg-surface-sunken hover:scale-[1.01] transition-all duration-150 border border-theme-subtle"
    >
      <h3 className="text-lg font-semibold text-theme-primary">{author.name}</h3>
      {author.description && (
        <p className="text-sm text-theme-tertiary mt-1 line-clamp-2">{author.description}</p>
      )}
    </button>
  )
}

function AudiobookCard({ audiobook, onClick }: { audiobook: Audiobook; onClick: () => void }) {
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
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = 'none'
            }}
          />
        </div>
        <div className="min-w-0 flex-1">
          <h3 className="text-lg font-semibold text-theme-primary truncate">{audiobook.title}</h3>
          {audiobook.metadata.narrator && (
            <p className="text-sm text-theme-tertiary mt-0.5">
              Narrated by {audiobook.metadata.narrators.length > 1
                ? audiobook.metadata.narrators.join(', ')
                : audiobook.metadata.narrator}
            </p>
          )}
          <div className="flex items-center gap-3 mt-1 text-xs text-theme-tertiary">
            {audiobook.year > 0 && <span>{audiobook.year}</span>}
            {audiobook.metadata.durationMinutes && (
              <span>{formatDuration(audiobook.metadata.durationMinutes)}</span>
            )}
            {audiobook.metadata.isAbridged && (
              <span className="px-1.5 py-0.5 bg-surface-sunken rounded text-theme-secondary">Abridged</span>
            )}
            {audiobook.metadata.seriesPosition && (
              <span className="text-theme-tertiary">Book {audiobook.metadata.seriesPosition}</span>
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
                  src={`${apiClient.getAudiobookCoverUrl(item.mediaItemId, 96)}`}
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

export function AudiobooksPage() {
  const navigate = useNavigate()
  const {
    authors,
    audiobooks,
    selectedAuthor,
    loading,
    error,
    loadAuthors,
    loadAudiobooks,
    loadAudiobooksByAuthor,
    selectAuthor,
  } = useAudiobookStore()

  useEffect(() => {
    loadAuthors()
    loadAudiobooks()
  }, [loadAuthors, loadAudiobooks])

  const handleAuthorClick = (author: Author) => {
    selectAuthor(author)
    loadAudiobooksByAuthor(author.id)
  }

  const handleBack = () => {
    selectAuthor(null)
    loadAudiobooks()
  }

  const handleBookClick = (audiobook: Audiobook) => {
    navigate(`/audiobooks/play/${audiobook.id}`)
  }

  return (
    <div className="max-w-5xl mx-auto p-4">
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          {selectedAuthor && (
            <Button variant="ghost" onClick={handleBack}>
              ← Back
            </Button>
          )}
          <h1 className="text-2xl font-serif font-semibold" style={{ color: 'rgb(var(--text-primary))' }}>
            {selectedAuthor ? selectedAuthor.name : 'Audiobooks'}
          </h1>
        </div>
      </div>

      {error && (
        <Card>
          <p className="text-red-400">{error}</p>
        </Card>
      )}

      {loading && (
        <div className="text-center py-12 text-theme-tertiary">Loading...</div>
      )}

      {!loading && !selectedAuthor && <ContinueListeningSection />}

      {!loading && !selectedAuthor && (
        <>
          <h2 className="text-xl font-bold text-theme-primary mb-4">Authors</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-8">
            {authors.map((author) => (
              <AuthorCard key={author.id} author={author} onClick={() => handleAuthorClick(author)} />
            ))}
          </div>

          <h2 className="text-xl font-bold text-theme-primary mb-4">All Audiobooks</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {audiobooks.map((book) => (
              <AudiobookCard key={book.id} audiobook={book} onClick={() => handleBookClick(book)} />
            ))}
          </div>
        </>
      )}

      {!loading && selectedAuthor && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {audiobooks.map((book) => (
            <AudiobookCard key={book.id} audiobook={book} onClick={() => handleBookClick(book)} />
          ))}
          {audiobooks.length === 0 && (
            <p className="text-theme-tertiary col-span-full text-center py-8">No audiobooks found for this author.</p>
          )}
        </div>
      )}
    </div>
  )
}
