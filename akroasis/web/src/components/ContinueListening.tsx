// Cross-media continue listening cards for Discovery page
import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useContinueStore } from '../stores/continueStore'
import { Card } from './Card'
import type { ContinueItem } from '../types'

function mediaTypeLabel(mediaType: string): string {
  switch (mediaType) {
    case 'audiobook': return 'Audiobook'
    case 'podcast': return 'Podcast'
    case 'music': return 'Music'
    default: return mediaType
  }
}

function ContinueCard({ item, onClick }: { item: ContinueItem; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="flex-shrink-0 w-48 text-left bg-accent-subtle rounded-lg hover:bg-surface-sunken transition-colors border border-theme-subtle overflow-hidden"
    >
      <div className="w-full h-28 bg-surface-sunken">
        {item.coverUrl && (
          <img
            src={item.coverUrl}
            alt={item.title}
            className="w-full h-full object-cover"
            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
          />
        )}
      </div>
      <div className="p-3">
        <p className="text-sm font-medium text-theme-primary truncate">{item.title}</p>
        <span className="inline-block mt-1 text-[10px] uppercase tracking-wider text-theme-tertiary bg-accent-subtle px-1.5 py-0.5 rounded">
          {mediaTypeLabel(item.mediaType)}
        </span>
        <div className="w-full bg-surface-sunken rounded-full h-1 mt-2">
          <div
            className="bg-accent h-1 rounded-full transition-all"
            style={{ width: `${Math.min(item.percentComplete, 100)}%` }}
          />
        </div>
        <p className="text-[11px] text-theme-tertiary mt-1">{Math.round(item.percentComplete)}%</p>
      </div>
    </button>
  )
}

export function ContinueListening() {
  const { items, fetchItems } = useContinueStore()
  const navigate = useNavigate()

  useEffect(() => {
    fetchItems(10)
  }, [fetchItems])

  if (items.length === 0) return null

  function handleClick(item: ContinueItem) {
    switch (item.mediaType) {
      case 'audiobook':
        navigate(`/audiobooks/play/${item.mediaItemId}`)
        break
      case 'podcast':
        navigate('/podcasts')
        break
      default:
        navigate('/player')
    }
  }

  return (
    <Card>
      <h2 className="text-lg font-semibold text-theme-secondary mb-4">Continue Listening</h2>
      <div className="flex gap-4 overflow-x-auto pb-2 -mx-1 px-1">
        {items.map((item) => (
          <ContinueCard key={item.mediaItemId} item={item} onClick={() => handleClick(item)} />
        ))}
      </div>
    </Card>
  )
}
