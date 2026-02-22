// Search results dropdown grouped by media type
import { getCoverArtUrl } from '../api/client'
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
      className="absolute top-full left-0 right-0 mt-1 bg-bronze-900 border border-bronze-700 rounded-lg shadow-xl max-h-96 overflow-y-auto z-50"
      role="listbox"
    >
      {TYPE_ORDER.map((type) => {
        const items = grouped.get(type)
        if (!items?.length) return null

        return (
          <div key={type}>
            <div className="px-3 py-1.5 text-[10px] font-semibold text-bronze-500 uppercase tracking-wider bg-bronze-900/80 sticky top-0">
              {TYPE_LABELS[type]}
            </div>
            {items.map((result) => {
              flatIndex++
              const isSelected = flatIndex === selectedIndex
              const coverSrc = result.type === 'track'
                ? getCoverArtUrl(result.id, 64)
                : result.coverUrl

              return (
                <button
                  key={`${result.type}-${result.id}`}
                  role="option"
                  aria-selected={isSelected}
                  className={`w-full flex items-center gap-3 px-3 py-2 text-left transition-colors ${
                    isSelected ? 'bg-bronze-700' : 'hover:bg-bronze-800'
                  }`}
                  onClick={() => onSelect(result)}
                >
                  <div className="w-8 h-8 flex-shrink-0 bg-bronze-700 rounded overflow-hidden">
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
                    <p className="text-sm text-bronze-100 truncate">{result.title}</p>
                    {result.subtitle && (
                      <p className="text-xs text-bronze-400 truncate">{result.subtitle}</p>
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
