// Signal path visualization: Source → Decode → EQ → Volume → Output
import { usePlayerStore } from '../stores/playerStore'
import { useEqStore } from '../stores/eqStore'

interface NodeChipProps {
  label: string
  active?: boolean
  muted?: boolean
}

function NodeChip({ label, active, muted }: NodeChipProps) {
  return (
    <div
      className={`px-2.5 py-1 rounded text-xs font-mono border transition-colors ${
        muted
          ? 'border-bronze-800 bg-bronze-950 text-bronze-700'
          : active
            ? 'border-bronze-500 bg-bronze-800 text-bronze-200'
            : 'border-bronze-700 bg-bronze-900 text-bronze-400'
      }`}
    >
      {label}
    </div>
  )
}

function Arrow() {
  return (
    <svg
      className="text-bronze-700 shrink-0"
      width="16"
      height="12"
      viewBox="0 0 16 12"
      fill="none"
      aria-hidden="true"
    >
      <path
        d="M0 6H12M12 6L7 1M12 6L7 11"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )
}

function formatSourceLabel(track: { format?: string; sampleRate?: number; bitDepth?: number } | null): string {
  if (!track?.format) return 'Source'
  const parts = [track.format.toUpperCase()]
  if (track.sampleRate) {
    parts.push(track.sampleRate >= 1000 ? `${(track.sampleRate / 1000).toFixed(1)}kHz` : `${track.sampleRate}Hz`)
  }
  if (track.bitDepth) parts.push(`${track.bitDepth}bit`)
  return parts.join(' ')
}

export function SignalPath() {
  const currentTrack = usePlayerStore((s) => s.currentTrack)
  const { enabled } = useEqStore()

  return (
    <div className="flex items-center gap-1.5 overflow-x-auto py-1">
      <NodeChip label={formatSourceLabel(currentTrack)} active={!!currentTrack} />
      <Arrow />
      <NodeChip label="Decode" />
      <Arrow />
      <NodeChip
        label={enabled ? 'EQ' : 'EQ (bypass)'}
        active={enabled}
        muted={!enabled}
      />
      <Arrow />
      <NodeChip label="Volume" />
      <Arrow />
      <NodeChip label="Output" />
    </div>
  )
}
