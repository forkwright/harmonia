// Recently added history entries
import { Card } from '../Card'
import type { HistoryEntry } from '../../types'

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

function eventTypeLabel(eventType: number): string {
  switch (eventType) {
    case 1: return 'Imported'
    case 2: return 'Downloaded'
    case 3: return 'Grabbed'
    case 4: return 'Deleted'
    default: return 'Event'
  }
}

interface Props {
  entries: HistoryEntry[]
}

export function RecentlyAddedSection({ entries }: Props) {
  if (entries.length === 0) {
    return (
      <Card>
        <h2 className="text-lg font-semibold text-bronze-300 mb-3">Recently Added</h2>
        <p className="text-bronze-500 text-sm">No recent history.</p>
      </Card>
    )
  }

  return (
    <Card>
      <h2 className="text-lg font-semibold text-bronze-300 mb-4">Recently Added</h2>
      <div className="space-y-2">
        {entries.map((entry) => (
          <div
            key={entry.id}
            className="flex items-start justify-between gap-4 p-3 bg-bronze-800/50 rounded-lg border border-bronze-700/30"
          >
            <div className="min-w-0 flex-1">
              <p className="text-bronze-100 text-sm font-medium truncate">
                {entry.sourceTitle}
              </p>
              <div className="flex items-center gap-2 mt-0.5">
                <span className="text-bronze-500 text-xs">
                  {eventTypeLabel(entry.eventType)}
                </span>
                {entry.quality.quality.name && (
                  <>
                    <span className="text-bronze-700 text-xs">&middot;</span>
                    <span className="text-bronze-500 text-xs">
                      {entry.quality.quality.name}
                    </span>
                  </>
                )}
              </div>
            </div>
            <span className="text-bronze-500 text-xs whitespace-nowrap shrink-0">
              {formatDate(entry.date)}
            </span>
          </div>
        ))}
      </div>
    </Card>
  )
}
