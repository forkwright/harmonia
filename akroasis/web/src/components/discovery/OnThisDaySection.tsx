// On This Day — sessions from the same date in previous years, enriched with track data
import { Card } from '../Card'
import { getCoverArtUrl } from '../../api/client'
import type { EnrichedSession } from '../../utils/discoveryStats'

function formatDurationMs(ms: number): string {
  const totalMinutes = Math.floor(ms / 60000)
  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  if (hours === 0) return `${minutes}m`
  return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`
}

function yearsAgo(dateStr: string): string {
  const then = new Date(dateStr)
  const now = new Date()
  const years = now.getFullYear() - then.getFullYear()
  return years === 1 ? '1 year ago' : `${years} years ago`
}

interface Props {
  sessions: EnrichedSession[]
}

export function OnThisDaySection({ sessions }: Props) {
  if (sessions.length === 0) {
    return (
      <Card>
        <h2 className="text-lg font-semibold text-bronze-300 mb-3">On This Day</h2>
        <p className="text-bronze-500 text-sm">Nothing in the archives for today's date.</p>
      </Card>
    )
  }

  return (
    <Card>
      <h2 className="text-lg font-semibold text-bronze-300 mb-4">On This Day</h2>
      <div className="space-y-3">
        {sessions.map((item) => (
          <div
            key={item.session.id}
            className="flex items-center gap-3 p-3 bg-bronze-800/50 rounded-lg border border-bronze-700/30"
          >
            {item.track && (
              <div className="w-10 h-10 flex-shrink-0 bg-bronze-700 rounded overflow-hidden">
                <img
                  src={getCoverArtUrl(item.track.id, 80)}
                  alt=""
                  className="w-full h-full object-cover"
                  onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
                />
              </div>
            )}
            <div className="min-w-0 flex-1">
              <p className="text-bronze-100 text-sm font-medium truncate">
                {item.track ? `${item.track.title} — ${item.track.artist}` : `Media #${item.session.mediaItemId}`}
              </p>
              <p className="text-bronze-400 text-xs mt-0.5">
                {item.session.deviceName} &middot; {formatDurationMs(item.session.durationMs)}
              </p>
            </div>
            <span className="text-bronze-500 text-xs whitespace-nowrap shrink-0">
              {yearsAgo(item.session.startedAt)}
            </span>
          </div>
        ))}
      </div>
    </Card>
  )
}
