// Top tracks, artists, and albums
import { Card } from '../Card'
import { getCoverArtUrl } from '../../api/client'
import type { TopItem } from '../../utils/discoveryStats'

function formatDurationMs(ms: number): string {
  const totalMinutes = Math.floor(ms / 60000)
  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  if (hours === 0) return `${minutes}m`
  return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`
}

function TopTrackRow({ item, rank }: { item: TopItem; rank: number }) {
  return (
    <div className="flex items-center gap-3 py-2">
      <span className="text-theme-tertiary text-xs w-5 text-right shrink-0">{rank}</span>
      {item.id && (
        <div className="w-8 h-8 flex-shrink-0 bg-surface-sunken rounded overflow-hidden">
          <img
            src={getCoverArtUrl(item.id, 64)}
            alt=""
            className="w-full h-full object-cover"
            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
          />
        </div>
      )}
      <div className="min-w-0 flex-1">
        <p className="text-sm text-theme-primary truncate">{item.name}</p>
      </div>
      <span className="text-theme-tertiary text-xs shrink-0">{item.count}x</span>
    </div>
  )
}

function TopItemRow({ item, rank }: { item: TopItem; rank: number }) {
  return (
    <div className="flex items-center gap-3 py-2">
      <span className="text-theme-tertiary text-xs w-5 text-right shrink-0">{rank}</span>
      <div className="min-w-0 flex-1">
        <p className="text-sm text-theme-primary truncate">{item.name}</p>
        <p className="text-xs text-theme-tertiary">{formatDurationMs(item.durationMs)}</p>
      </div>
      <span className="text-theme-tertiary text-xs shrink-0">{item.count} plays</span>
    </div>
  )
}

interface Props {
  topTracks: TopItem[]
  topArtists: TopItem[]
  topAlbums: TopItem[]
}

export function TopListsSection({ topTracks, topArtists, topAlbums }: Props) {
  const hasData = topTracks.length > 0 || topArtists.length > 0 || topAlbums.length > 0
  if (!hasData) return null

  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
      {topTracks.length > 0 && (
        <Card>
          <h3 className="text-sm font-semibold text-theme-tertiary uppercase tracking-wider mb-3">Top Tracks</h3>
          <div className="divide-y divide-theme-subtle">
            {topTracks.map((item, i) => (
              <TopTrackRow key={item.name} item={item} rank={i + 1} />
            ))}
          </div>
        </Card>
      )}

      {topArtists.length > 0 && (
        <Card>
          <h3 className="text-sm font-semibold text-theme-tertiary uppercase tracking-wider mb-3">Top Artists</h3>
          <div className="divide-y divide-theme-subtle">
            {topArtists.map((item, i) => (
              <TopItemRow key={item.name} item={item} rank={i + 1} />
            ))}
          </div>
        </Card>
      )}

      {topAlbums.length > 0 && (
        <Card>
          <h3 className="text-sm font-semibold text-theme-tertiary uppercase tracking-wider mb-3">Top Albums</h3>
          <div className="divide-y divide-theme-subtle">
            {topAlbums.map((item, i) => (
              <TopItemRow key={item.name} item={item} rank={i + 1} />
            ))}
          </div>
        </Card>
      )}
    </div>
  )
}
