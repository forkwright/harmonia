// Synchronized lyrics display component
import { useEffect, useRef } from 'react'
import type { LrcLine } from '../utils/lrcParser'
import type { LyricsStatus } from '../hooks/useLyrics'

interface LyricsDisplayProps {
  status: LyricsStatus
  lines: LrcLine[]
  plainLyrics: string | null
  activeLine: number
}

export function LyricsDisplay({ status, lines, plainLyrics, activeLine }: LyricsDisplayProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const activeRef = useRef<HTMLParagraphElement>(null)

  useEffect(() => {
    if (!activeRef.current || !containerRef.current) return
    activeRef.current.scrollIntoView({ block: 'center', behavior: 'smooth' })
  }, [activeLine])

  if (status === 'loading') {
    return (
      <div className="flex justify-center py-8">
        <div className="w-5 h-5 border-2 border-bronze-600 border-t-bronze-300 rounded-full animate-spin" />
      </div>
    )
  }

  if (status === 'not-found' || status === 'idle') {
    return (
      <p className="text-center text-bronze-600 text-sm py-8">No lyrics found</p>
    )
  }

  if (status === 'error') {
    return (
      <p className="text-center text-bronze-600 text-sm py-8">Could not load lyrics</p>
    )
  }

  if (status === 'plain' && plainLyrics) {
    return (
      <div className="max-h-64 overflow-y-auto px-2 py-4 space-y-1 scrollbar-hide">
        {plainLyrics.split('\n').map((line, i) => (
          <p
            key={i}
            className="text-center text-bronze-300 text-sm leading-relaxed whitespace-pre-wrap"
          >
            {line || '\u00A0'}
          </p>
        ))}
      </div>
    )
  }

  if (status === 'synced' && lines.length > 0) {
    return (
      <div
        ref={containerRef}
        className="max-h-64 overflow-y-auto px-2 py-4 space-y-2 scrollbar-hide"
      >
        {lines.map((line, i) => {
          const isActive = i === activeLine
          const isPast = i < activeLine
          return (
            <p
              key={i}
              ref={isActive ? activeRef : undefined}
              className={[
                'text-center text-sm leading-relaxed transition-all duration-300 whitespace-pre-wrap',
                isActive
                  ? 'text-bronze-100 font-semibold scale-105 origin-center'
                  : isPast
                    ? 'text-bronze-600'
                    : 'text-bronze-500',
              ].join(' ')}
            >
              {line.text}
            </p>
          )
        })}
      </div>
    )
  }

  return null
}
