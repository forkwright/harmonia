// Repeat mode cycle button — Off → All → One → Shuffle+Repeat
import type { RepeatMode } from '../stores/playerStore'
import { usePlayerStore } from '../stores/playerStore'

const MODE_LABELS: Record<RepeatMode, string> = {
  'off': 'Repeat off',
  'all': 'Repeat all',
  'one': 'Repeat one',
  'shuffle-repeat': 'Shuffle and repeat',
}

function RepeatIcon({ mode }: { readonly mode: RepeatMode }) {
  if (mode === 'shuffle-repeat') {
    return (
      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
        <path fillRule="evenodd" d="M2.5 7.5a5 5 0 0110 0v.793l-1.146-1.147a.5.5 0 10-.708.708l2 2a.5.5 0 00.708 0l2-2a.5.5 0 00-.708-.708L13.5 8.293V7.5a6 6 0 00-12 0v5a6 6 0 0012 0v-.793l1.146 1.147a.5.5 0 00.708-.708l-2-2a.5.5 0 00-.708 0l-2 2a.5.5 0 00.708.708L12.5 11.707v.793a5 5 0 01-10 0v-5z" clipRule="evenodd"/>
      </svg>
    )
  }

  return (
    <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
      <path fillRule="evenodd" d="M4 3a1 1 0 00-1 1v8a4 4 0 004 4h4.586l-1.293 1.293a1 1 0 101.414 1.414l3-3a1 1 0 000-1.414l-3-3a1 1 0 00-1.414 1.414L12.586 14H7a2 2 0 01-2-2V4a1 1 0 00-1-1zm12 14a1 1 0 001-1V8a4 4 0 00-4-4H8.414l1.293-1.293a1 1 0 10-1.414-1.414l-3 3a1 1 0 000 1.414l3 3a1 1 0 001.414-1.414L8.414 6H13a2 2 0 012 2v8a1 1 0 001 1z" clipRule="evenodd"/>
    </svg>
  )
}

export function RepeatButton() {
  const repeatMode = usePlayerStore((s) => s.repeatMode)
  const cycleRepeatMode = usePlayerStore((s) => s.cycleRepeatMode)

  const isActive = repeatMode !== 'off'

  return (
    <button
      onClick={cycleRepeatMode}
      className={`relative flex items-center gap-1 px-2 py-1 rounded text-xs transition-colors ${
        isActive
          ? 'text-theme-primary bg-surface-sunken'
          : 'text-theme-tertiary hover:text-theme-secondary'
      }`}
      title={MODE_LABELS[repeatMode]}
      aria-label={MODE_LABELS[repeatMode]}
    >
      <RepeatIcon mode={repeatMode} />
      {repeatMode === 'one' && (
        <span className="absolute -top-1 -right-1 bg-accent text-theme-primary rounded-full w-3.5 h-3.5 flex items-center justify-center text-[9px] font-bold">
          1
        </span>
      )}
    </button>
  )
}
