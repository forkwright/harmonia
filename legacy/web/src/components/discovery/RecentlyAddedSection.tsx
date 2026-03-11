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
        <h2 className="text-lg font-semibold text-theme-secondary mb-3">Recently Added</h2>
        <p className="text-theme-tertiary text-sm">No recent history.</p>
      </Card>
    )
  }

  return (
    <Card>
      <h2 className="text-lg font-semibold text-theme-secondary mb-4">Recently Added</h2>
      <div className="space-y-2">
        {entries.map((entry) => (
          <div
            key={entry.id}
            className="flex items-start justify-between gap-4 p-3 bg-accent-subtle rounded-lg border border-theme-subtle"
          >
            <div className="min-w-0 flex-1">
              <p className="text-theme-primary text-sm font-medium truncate">
                {entry.sourceTitle}
              </p>
              <div className="flex items-center gap-2 mt-0.5">
                <span className="text-theme-tertiary text-xs">
                  {eventTypeLabel(entry.eventType)}
                </span>
                {entry.quality.quality.name && (
                  <>
                    <span className="text-theme-muted text-xs">&middot;</span>
                    <span className="text-theme-tertiary text-xs">
                      {entry.quality.quality.name}
                    </span>
                  </>
                )}
              </div>
            </div>
            <span className="text-theme-tertiary text-xs whitespace-nowrap shrink-0">
              {formatDate(entry.date)}
            </span>
          </div>
        ))}
      </div>
    </Card>
  )
}
