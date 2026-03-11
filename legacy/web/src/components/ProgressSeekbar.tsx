// Simple progress bar seekbar — no Web Audio API dependency
import { useRef, useCallback } from 'react'

interface ProgressSeekbarProps {
  duration: number   // ms
  position: number   // ms
  onSeek: (ms: number) => void
  disabled?: boolean
}

export function ProgressSeekbar({ duration, position, onSeek, disabled }: ProgressSeekbarProps) {
  const barRef = useRef<HTMLDivElement>(null)
  const dragging = useRef(false)

  const seekFromEvent = useCallback((clientX: number) => {
    if (disabled || !duration || !barRef.current) return
    const rect = barRef.current.getBoundingClientRect()
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width))
    onSeek(ratio * duration)
  }, [disabled, duration, onSeek])

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    if (disabled || !duration) return
    dragging.current = true
    ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
    seekFromEvent(e.clientX)
  }, [disabled, duration, seekFromEvent])

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (!dragging.current) return
    seekFromEvent(e.clientX)
  }, [seekFromEvent])

  const handlePointerUp = useCallback(() => {
    dragging.current = false
  }, [])

  const progress = duration > 0 ? Math.min(position / duration, 1) * 100 : 0

  return (
    <div
      ref={barRef}
      className={`relative w-full h-10 flex items-center ${disabled ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer'} group`}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      role="slider"
      aria-label="Seek"
      aria-valuemin={0}
      aria-valuemax={duration}
      aria-valuenow={position}
    >
      {/* Track */}
      <div
        className="w-full h-1.5 rounded-full overflow-hidden group-hover:h-2.5 transition-all duration-150"
        style={{ backgroundColor: 'rgb(var(--border-subtle))' }}
      >
        {/* Played portion */}
        <div
          className="h-full rounded-full transition-[width] duration-100 ease-linear"
          style={{
            width: `${progress}%`,
            backgroundColor: 'rgb(var(--accent-primary))',
          }}
        />
      </div>

      {/* Thumb — visible on hover */}
      <div
        className="absolute w-3.5 h-3.5 rounded-full opacity-0 group-hover:opacity-100 transition-opacity shadow-md pointer-events-none"
        style={{
          left: `calc(${progress}% - 7px)`,
          backgroundColor: 'rgb(var(--accent-primary))',
          boxShadow: '0 0 4px rgb(var(--accent-primary) / 0.4)',
        }}
      />
    </div>
  )
}
