// Annual listening retrospective
import { Card } from '../Card'
import type { YearInReview, TopItem } from '../../utils/discoveryStats'

const MONTH_LABELS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']

function formatDurationMs(ms: number): string {
  const totalMinutes = Math.floor(ms / 60000)
  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  if (hours === 0) return `${minutes}m`
  return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`
}

function StatChip({ label, value }: { label: string; value: string }) {
  return (
    <div className="text-center">
      <div className="text-lg font-bold text-theme-primary">{value}</div>
      <div className="text-xs text-theme-tertiary">{label}</div>
    </div>
  )
}

function MonthlyChart({ breakdown }: { breakdown: YearInReview['monthlyBreakdown'] }) {
  const maxMs = Math.max(...breakdown.map((m) => m.durationMs), 1)

  return (
    <div className="flex items-end gap-1 h-20">
      {breakdown.map((m) => {
        const pct = (m.durationMs / maxMs) * 100
        return (
          <div key={m.month} className="flex-1 flex flex-col items-center gap-1">
            <div
              className="w-full rounded-t transition-all"
              style={{
                height: `${Math.max(pct, m.durationMs > 0 ? 4 : 0)}%`,
                background: pct > 75
                  ? 'rgb(180, 111, 63)'
                  : pct > 25
                    ? 'rgb(150, 90, 50)'
                    : 'rgb(100, 70, 45)',
              }}
              title={`${MONTH_LABELS[m.month]}: ${formatDurationMs(m.durationMs)}, ${m.sessions} sessions`}
            />
            <span className="text-[10px] text-theme-muted">{MONTH_LABELS[m.month]}</span>
          </div>
        )
      })}
    </div>
  )
}

function CompactTopList({ title, items }: { title: string; items: TopItem[] }) {
  if (items.length === 0) return null

  return (
    <div>
      <h4 className="text-xs font-semibold text-theme-tertiary uppercase tracking-wider mb-1.5">{title}</h4>
      <div className="space-y-1">
        {items.map((item, i) => (
          <div key={item.name} className="flex items-center gap-2 text-sm">
            <span className="text-theme-muted text-xs w-4 text-right">{i + 1}</span>
            <span className="text-theme-primary truncate flex-1">{item.name}</span>
            <span className="text-theme-tertiary text-xs shrink-0">{item.count}x</span>
          </div>
        ))}
      </div>
    </div>
  )
}

interface Props {
  review: YearInReview
}

export function YearInReviewSection({ review }: Props) {
  if (review.totalSessions === 0) return null

  return (
    <Card>
      <h3 className="text-sm font-semibold text-theme-tertiary uppercase tracking-wider mb-4">
        Year in Review — {review.year}
      </h3>

      <div className="flex justify-around mb-5">
        <StatChip label="Listening" value={formatDurationMs(review.totalMs)} />
        <StatChip label="Sessions" value={String(review.totalSessions)} />
        <StatChip label="Active Days" value={String(review.activeDays)} />
        {review.mostActiveMonth && (
          <StatChip label="Peak Month" value={MONTH_LABELS[review.mostActiveMonth.month]} />
        )}
      </div>

      <div className="mb-5">
        <MonthlyChart breakdown={review.monthlyBreakdown} />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <CompactTopList title="Top Tracks" items={review.topTracks} />
        <CompactTopList title="Top Artists" items={review.topArtists} />
        <CompactTopList title="Top Albums" items={review.topAlbums} />
      </div>
    </Card>
  )
}
