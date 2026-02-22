// Listening DNA — 5-dimension taste profile
import { Card } from '../Card'
import type { ListeningDna } from '../../utils/discoveryStats'

const DAY_NAMES = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']

function DimensionCard({ title, label, value, maxValue, subtitle }: {
  title: string; label: string; value: number; maxValue: number; subtitle: string
}) {
  const pct = maxValue > 0 ? Math.min(100, (value / maxValue) * 100) : 0
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <span className="text-xs text-bronze-500">{title}</span>
        <span className="text-xs font-medium text-bronze-200">{label}</span>
      </div>
      <div className="h-1.5 bg-bronze-800 rounded-full overflow-hidden">
        <div
          className="h-full bg-bronze-500 rounded-full transition-all"
          style={{ width: `${pct}%` }}
        />
      </div>
      <p className="text-[10px] text-bronze-600">{subtitle}</p>
    </div>
  )
}

function Sparkline({ data }: { data: number[] }) {
  if (data.length === 0) return null
  const max = Math.max(...data, 1)
  const w = 120
  const h = 24
  const points = data.map((v, i) => `${(i / (data.length - 1)) * w},${h - (v / max) * h}`)
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} className="text-bronze-500">
      <polyline
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        points={points.join(' ')}
      />
    </svg>
  )
}

function formatHour(h: number): string {
  const ampm = h >= 12 ? 'PM' : 'AM'
  const hour12 = h % 12 || 12
  return `${hour12}${ampm}`
}

export function ListeningDnaSection({ dna }: { dna: ListeningDna | null }) {
  if (!dna || dna.artistDiversity.totalPlays < 10) return null

  const { artistDiversity, albumDepth, sessionPatterns, formatPreferences, listeningVelocity } = dna

  return (
    <Card>
      <h2 className="text-lg font-bold text-bronze-100 mb-4">Listening DNA</h2>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <DimensionCard
          title="Artist Diversity"
          label={artistDiversity.label}
          value={artistDiversity.entropy}
          maxValue={7}
          subtitle={`${artistDiversity.uniqueArtists} artists across ${artistDiversity.totalPlays} plays`}
        />

        <DimensionCard
          title="Album Depth"
          label={albumDepth.label}
          value={albumDepth.completionRate}
          maxValue={1}
          subtitle={`${albumDepth.albumsCompleted}/${albumDepth.albumsStarted} albums completed · ${albumDepth.avgTracksPerAlbum.toFixed(1)} tracks/album`}
        />

        <DimensionCard
          title="Session Style"
          label={sessionPatterns.label}
          value={sessionPatterns.avgSessionMinutes}
          maxValue={120}
          subtitle={`Peak: ${formatHour(sessionPatterns.peakHour)} on ${DAY_NAMES[sessionPatterns.peakDay]} · ${sessionPatterns.sessionsPerWeek.toFixed(1)}/week`}
        />

        <DimensionCard
          title="Format Preference"
          label={formatPreferences.label}
          value={formatPreferences.losslessPct}
          maxValue={100}
          subtitle={`${formatPreferences.losslessPct.toFixed(0)}% lossless · mostly ${formatPreferences.dominantFormat.toUpperCase()}`}
        />

        <div className="md:col-span-2 space-y-1.5">
          <div className="flex items-center justify-between">
            <span className="text-xs text-bronze-500">Listening Velocity</span>
            <span className="text-xs font-medium text-bronze-200">{listeningVelocity.label}</span>
          </div>
          <div className="flex items-center gap-3">
            <Sparkline data={listeningVelocity.tracksPerWeek} />
            <span className="text-[10px] text-bronze-600">
              {listeningVelocity.trend === 'accelerating' ? '↑ Picking up' :
               listeningVelocity.trend === 'decelerating' ? '↓ Slowing down' : '→ Steady'}
              {' '}(12 weeks)
            </span>
          </div>
        </div>
      </div>
    </Card>
  )
}
