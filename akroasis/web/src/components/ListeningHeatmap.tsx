// GitHub-style listening activity heatmap — pure SVG, no dependencies
import type { DayActivity } from '../utils/discoveryStats'

const CELL_SIZE = 13
const GAP = 2
const STEP = CELL_SIZE + GAP

const MONTH_LABELS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
const DAY_LABELS = ['Mon', '', 'Wed', '', 'Fri', '', '']

function getColor(minutes: number): string {
  if (minutes === 0) return 'rgb(37, 28, 23)'     // bronze-800/50 equivalent
  if (minutes <= 15) return 'rgb(113, 73, 48)'     // bronze-700
  if (minutes <= 45) return 'rgb(147, 93, 58)'     // bronze-600
  if (minutes <= 90) return 'rgb(180, 111, 63)'    // bronze-500
  return 'rgb(205, 140, 88)'                        // bronze-400
}

interface ListeningHeatmapProps {
  dailyActivity: DayActivity[]
}

export function ListeningHeatmap({ dailyActivity }: ListeningHeatmapProps) {
  const dayMap = new Map<string, number>()
  for (const d of dailyActivity) {
    dayMap.set(d.date, d.durationMinutes)
  }

  const firstDate = dailyActivity[0]?.date
  if (!firstDate) return null

  const start = new Date(firstDate + 'T00:00:00')
  const startDow = start.getDay()

  const labelWidth = 28
  const headerHeight = 16
  const cols = 52
  const rows = 7
  const svgWidth = labelWidth + cols * STEP
  const svgHeight = headerHeight + rows * STEP

  const cells: Array<{ x: number; y: number; color: string; title: string }> = []
  const cursor = new Date(start)

  for (let i = 0; i < dailyActivity.length; i++) {
    const dayIndex = startDow + i
    const col = Math.floor(dayIndex / 7)
    const row = dayIndex % 7

    const key = cursor.toISOString().slice(0, 10)
    const minutes = dayMap.get(key) ?? 0
    const label = minutes > 0
      ? `${cursor.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}: ${minutes}m`
      : cursor.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })

    cells.push({
      x: labelWidth + col * STEP,
      y: headerHeight + row * STEP,
      color: getColor(minutes),
      title: label,
    })

    cursor.setDate(cursor.getDate() + 1)
  }

  const monthPositions: Array<{ label: string; x: number }> = []
  const seen = new Set<number>()
  const walker = new Date(start)
  for (let i = 0; i < dailyActivity.length; i++) {
    const m = walker.getMonth()
    if (!seen.has(m)) {
      seen.add(m)
      const dayIndex = startDow + i
      const col = Math.floor(dayIndex / 7)
      monthPositions.push({ label: MONTH_LABELS[m], x: labelWidth + col * STEP })
    }
    walker.setDate(walker.getDate() + 1)
  }

  return (
    <div className="overflow-x-auto">
      <svg
        width={svgWidth}
        height={svgHeight}
        className="block"
        role="img"
        aria-label="Listening activity heatmap"
      >
        {monthPositions.map((mp) => (
          <text
            key={`month-${mp.label}-${mp.x}`}
            x={mp.x}
            y={10}
            className="fill-bronze-500"
            fontSize={10}
            fontFamily="sans-serif"
          >
            {mp.label}
          </text>
        ))}

        {DAY_LABELS.map((label, i) => label ? (
          <text
            key={`day-${i}`}
            x={0}
            y={headerHeight + i * STEP + CELL_SIZE - 2}
            className="fill-bronze-500"
            fontSize={9}
            fontFamily="sans-serif"
          >
            {label}
          </text>
        ) : null)}

        {cells.map((cell, i) => (
          <rect
            key={i}
            x={cell.x}
            y={cell.y}
            width={CELL_SIZE}
            height={CELL_SIZE}
            rx={2}
            fill={cell.color}
          >
            <title>{cell.title}</title>
          </rect>
        ))}
      </svg>
    </div>
  )
}
