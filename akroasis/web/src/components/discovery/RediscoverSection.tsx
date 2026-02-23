// Rediscover — forgotten favorites with 2+ plays, last played 6+ months ago
import { Card } from '../Card'
import { getCoverArtUrl } from '../../api/client'
import type { Track } from '../../types'

interface RediscoverItem {
  track: Track
  playCount: number
  lastPlayed: Date
}

function monthsAgo(date: Date): string {
  const now = new Date()
  const months = (now.getFullYear() - date.getFullYear()) * 12 + (now.getMonth() - date.getMonth())
  if (months >= 12) {
    const years = Math.floor(months / 12)
    return years === 1 ? '1 year ago' : `${years} years ago`
  }
  return months === 1 ? '1 month ago' : `${months} months ago`
}

interface Props {
  candidates: RediscoverItem[]
  onPlay?: (track: Track) => void
}

export function RediscoverSection({ candidates, onPlay }: Props) {
  if (candidates.length === 0) {
    return (
      <Card>
        <h2 className="text-lg font-semibold text-theme-secondary mb-3">Rediscover</h2>
        <p className="text-theme-tertiary text-sm">
          Keep listening! Suggestions appear after you've built some history.
        </p>
      </Card>
    )
  }

  return (
    <Card>
      <h2 className="text-lg font-semibold text-theme-secondary mb-4">Rediscover</h2>
      <div className="flex gap-4 overflow-x-auto pb-2 -mx-1 px-1">
        {candidates.map((item) => (
          <button
            key={item.track.id}
            onClick={() => onPlay?.(item.track)}
            className="flex-shrink-0 w-36 text-left group"
          >
            <div className="relative w-36 h-36 bg-surface-sunken rounded-lg overflow-hidden mb-2">
              <img
                src={getCoverArtUrl(item.track.id, 256)}
                alt={item.track.title}
                className="w-full h-full object-cover"
                onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
              />
              <div className="absolute inset-0 bg-black/0 group-hover:bg-black/40 transition-colors flex items-center justify-center">
                <svg
                  className="w-10 h-10 text-white opacity-0 group-hover:opacity-100 transition-opacity"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                >
                  <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clipRule="evenodd" />
                </svg>
              </div>
            </div>
            <p className="text-sm font-medium text-theme-primary truncate">{item.track.title}</p>
            <p className="text-xs text-theme-tertiary truncate">{item.track.artist}</p>
            <div className="flex items-center gap-2 mt-1">
              <span className="text-[10px] text-theme-tertiary bg-accent-subtle px-1.5 py-0.5 rounded">
                {item.playCount}x
              </span>
              <span className="text-[10px] text-theme-tertiary">{monthsAgo(item.lastPlayed)}</span>
            </div>
          </button>
        ))}
      </div>
    </Card>
  )
}
