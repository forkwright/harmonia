// Listening stats dashboard + heatmap
import { Card } from '../Card'
import { ListeningHeatmap } from '../ListeningHeatmap'
import type { ListeningStats, DayActivity } from '../../utils/discoveryStats'

function formatDurationMs(ms: number): string {
  const totalMinutes = Math.floor(ms / 60000)
  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  if (hours === 0 && minutes === 0) return '—'
  if (hours === 0) return `${minutes}m`
  return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`
}

interface Props {
  stats: ListeningStats
  dailyActivity: DayActivity[]
}

export function ListeningStatsSection({ stats, dailyActivity }: Props) {
  return (
    <Card>
      <h2 className="text-lg font-semibold text-theme-secondary mb-4">Listening Stats</h2>

      <div className="mb-4">
        <p className="text-theme-tertiary text-xs uppercase tracking-wide mb-1">All Time</p>
        <p className="text-theme-primary text-3xl font-bold">{formatDurationMs(stats.allTimeMs)}</p>
        <p className="text-theme-tertiary text-xs mt-1">
          {stats.totalSessions} sessions &middot; {stats.activeDays} active days
        </p>
      </div>

      <div className="grid grid-cols-3 gap-3 mb-6">
        <div className="p-3 bg-accent-subtle rounded-lg border border-theme-subtle">
          <p className="text-theme-tertiary text-xs uppercase tracking-wide mb-1">This Month</p>
          <p className="text-theme-primary text-lg font-semibold">{formatDurationMs(stats.thisMonthMs)}</p>
        </div>
        <div className="p-3 bg-accent-subtle rounded-lg border border-theme-subtle">
          <p className="text-theme-tertiary text-xs uppercase tracking-wide mb-1">This Week</p>
          <p className="text-theme-primary text-lg font-semibold">{formatDurationMs(stats.thisWeekMs)}</p>
        </div>
        <div className="p-3 bg-accent-subtle rounded-lg border border-theme-subtle">
          <p className="text-theme-tertiary text-xs uppercase tracking-wide mb-1">Today</p>
          <p className="text-theme-primary text-lg font-semibold">{formatDurationMs(stats.todayMs)}</p>
        </div>
      </div>

      <div className="mb-2">
        <p className="text-theme-tertiary text-xs uppercase tracking-wide mb-3">Activity</p>
        <ListeningHeatmap dailyActivity={dailyActivity} />
      </div>

      <div className="flex items-center gap-2 mt-3 justify-end">
        <span className="text-theme-tertiary text-[10px]">Less</span>
        {[0, 10, 30, 60, 100].map((m) => (
          <div
            key={m}
            className="w-3 h-3 rounded-sm"
            style={{ backgroundColor: m === 0 ? 'rgb(37, 28, 23)' : m <= 15 ? 'rgb(113, 73, 48)' : m <= 45 ? 'rgb(147, 93, 58)' : m <= 90 ? 'rgb(180, 111, 63)' : 'rgb(205, 140, 88)' }}
          />
        ))}
        <span className="text-theme-tertiary text-[10px]">More</span>
      </div>
    </Card>
  )
}
