// Search results dropdown grouped by media type
import { getCoverArtUrl, authenticateUrl } from '../api/client'
import type { UnifiedSearchResult, SearchResultType } from '../types'

const TYPE_LABELS: Record<SearchResultType, string> = {
  track: 'Music',
  audiobook: 'Audiobooks',
  podcast: 'Podcasts',
}

const TYPE_ORDER: SearchResultType[] = ['track', 'audiobook', 'podcast']

interface Props {
  results: UnifiedSearchResult[]
  selectedIndex: number
  onSelect: (result: UnifiedSearchResult) => void
}

export function SearchDropdown({ results, selectedIndex, onSelect }: Props) {
  if (results.length === 0) return null

  const grouped = new Map<SearchResultType, UnifiedSearchResult[]>()
  for (const r of results) {
    const group = grouped.get(r.type) ?? []
    group.push(r)
    grouped.set(r.type, group)
  }

  let flatIndex = -1

  return (
    <div
      className="absolute top-full left-0 right-0 mt-1 rounded-lg shadow-xl max-h-96 overflow-y-auto z-50"
      style={{
        backgroundColor: 'rgb(var(--surface-overlay))',
        border: '1px solid rgb(var(--border-default))',
      }}
      role="listbox"
    >
      {TYPE_ORDER.map((type) => {
        const items = grouped.get(type)
        if (!items?.length) return null

        return (
          <div key={type}>
            <div
              className="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider sticky top-0"
              style={{
                color: 'rgb(var(--text-muted))',
                backgroundColor: 'rgb(var(--surface-overlay))',
              }}
            >
              {TYPE_LABELS[type]}
            </div>
            {items.map((result) => {
              flatIndex++
              const isSelected = flatIndex === selectedIndex
              const coverSrc = authenticateUrl(
                result.type === 'track'
                  ? getCoverArtUrl(result.id, 64)
                  : result.coverUrl
              )

              return (
                <button
                  key={`${result.type}-${result.id}`}
                  role="option"
                  aria-selected={isSelected}
                  className="w-full flex items-center gap-3 px-3 py-2 text-left transition-colors"
                  style={{
                    backgroundColor: isSelected ? 'rgb(var(--accent-primary) / 0.1)' : undefined,
                  }}
                  onClick={() => onSelect(result)}
                >
                  <div
                    className="w-8 h-8 flex-shrink-0 rounded overflow-hidden"
                    style={{ backgroundColor: 'rgb(var(--surface-sunken))' }}
                  >
                    {coverSrc && (
                      <img
                        src={coverSrc}
                        alt=""
                        className="w-full h-full object-cover"
                        onError={(e) => { (e.target as HTMLImageElement).style.display = 'none' }}
                      />
                    )}
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="text-sm truncate" style={{ color: 'rgb(var(--text-primary))' }}>{result.title}</p>
                    {result.subtitle && (
                      <p className="text-xs truncate" style={{ color: 'rgb(var(--text-tertiary))' }}>{result.subtitle}</p>
                    )}
                  </div>
                </button>
              )
            })}
          </div>
        )
      })}
    </div>
  )
}
