// Signal path visualization with Roon-inspired quality coding
// Each node is colored by the quality tier at that point in the chain
import { useState } from 'react'
import { usePlayerStore } from '../stores/playerStore'
import { useEqStore } from '../stores/eqStore'
import { useCompressorStore } from '../stores/compressorStore'
import { useReplayGainStore } from '../stores/replayGainStore'
import { useMetaxisStore } from '../stores/metaxisStore'
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer'

// ─── Quality Tier System ────────────────────────────────────────

export type QualityTier = 'enhanced' | 'lossless' | 'high' | 'standard' | 'low'

const TIER_RANK: Record<QualityTier, number> = {
  enhanced: 4,
  lossless: 3,
  high: 2,
  standard: 1,
  low: 0,
}

const TIER_LABEL: Record<QualityTier, string> = {
  enhanced: 'Enhanced',
  lossless: 'Lossless',
  high: 'High Quality',
  standard: 'Standard',
  low: 'Low Quality',
}

function tierMin(a: QualityTier, b: QualityTier): QualityTier {
  return TIER_RANK[a] <= TIER_RANK[b] ? a : b
}

function tierMax(a: QualityTier, b: QualityTier): QualityTier {
  return TIER_RANK[a] >= TIER_RANK[b] ? a : b
}

// ─── Source Quality Assessment ──────────────────────────────────

export function getSourceTier(track: {
  format?: string
  sampleRate?: number
  bitDepth?: number
} | null): QualityTier {
  if (!track?.format) return 'standard'

  const fmt = track.format.toLowerCase()
  const sr = track.sampleRate ?? 0
  const bd = track.bitDepth ?? 0
  const isLossless = ['flac', 'wav', 'alac', 'aiff', 'dsd', 'dsf', 'dff', 'wv', 'ape'].includes(fmt)

  if (isLossless && (sr > 48000 || bd > 16)) return 'enhanced'
  if (isLossless) return 'lossless'
  // AAC/OGG at high bitrate
  if (['aac', 'ogg', 'opus'].includes(fmt)) return 'high'
  // MP3, low-bitrate
  return 'standard'
}

// ─── Node Types ─────────────────────────────────────────────────

interface PipelineNode {
  id: string
  label: string
  detail?: string
  tier: QualityTier
  active: boolean
  bypassed?: boolean
  warning?: string
}

function buildPipeline(
  track: { format?: string; sampleRate?: number; bitDepth?: number } | null,
  eqEnabled: boolean,
  compressorEnabled: boolean,
  rgMode: string,
  limiterEnabled: boolean,
  crossfadeMode: string,
  outputSampleRate: number | null,
): PipelineNode[] {
  const nodes: PipelineNode[] = []
  let currentTier = getSourceTier(track)

  // Source
  const srcParts: string[] = []
  if (track?.format) srcParts.push(track.format.toUpperCase())
  if (track?.sampleRate) {
    srcParts.push(track.sampleRate >= 1000
      ? `${(track.sampleRate / 1000).toFixed(1)}kHz`
      : `${track.sampleRate}Hz`)
  }
  if (track?.bitDepth) srcParts.push(`${track.bitDepth}bit`)

  nodes.push({
    id: 'source',
    label: srcParts.join(' ') || 'Source',
    tier: currentTier,
    active: !!track,
  })

  // Crossfade (Metaxis) — if active, appears before decode
  if (crossfadeMode !== 'off') {
    nodes.push({
      id: 'crossfade',
      label: 'Metaxis',
      detail: 'Crossfade',
      tier: currentTier,
      active: true,
    })
  }

  // Decode
  nodes.push({
    id: 'decode',
    label: 'Decode',
    tier: currentTier,
    active: true,
  })

  // EQ — intentional enhancement: elevate to at least 'high' if lossless+
  if (eqEnabled) {
    currentTier = tierMax(currentTier, 'high')
    if (TIER_RANK[getSourceTier(track)] >= TIER_RANK['lossless']) {
      currentTier = 'enhanced'
    }
  }
  nodes.push({
    id: 'eq',
    label: 'EQ',
    detail: eqEnabled ? '10-band' : undefined,
    tier: currentTier,
    active: eqEnabled,
    bypassed: !eqEnabled,
  })

  // Compressor — doesn't inherently degrade
  nodes.push({
    id: 'compressor',
    label: 'Dynamics',
    tier: currentTier,
    active: compressorEnabled,
    bypassed: !compressorEnabled,
  })

  // ReplayGain — doesn't degrade (float domain gain)
  const rgActive = rgMode !== 'off'
  nodes.push({
    id: 'replaygain',
    label: rgActive ? `RG (${rgMode})` : 'RG',
    tier: currentTier,
    active: rgActive,
    bypassed: !rgActive,
  })

  // Limiter — doesn't degrade, protective
  const limActive = limiterEnabled && rgActive
  nodes.push({
    id: 'limiter',
    label: 'Limiter',
    tier: currentTier,
    active: limActive,
    bypassed: !limActive,
  })

  // Volume — gain in float domain is lossless
  nodes.push({
    id: 'volume',
    label: 'Volume',
    tier: currentTier,
    active: true,
  })

  // Output — browser resampling check
  const outputRate = outputSampleRate ?? 48000
  const sourceRate = track?.sampleRate ?? 44100
  const isResampled = sourceRate !== outputRate && sourceRate > 0
  if (isResampled) {
    currentTier = tierMin(currentTier, 'standard')
  }

  nodes.push({
    id: 'output',
    label: `Output ${outputRate >= 1000 ? `${(outputRate / 1000).toFixed(1)}kHz` : `${outputRate}Hz`}`,
    tier: currentTier,
    active: true,
    warning: isResampled
      ? `Browser resamples ${sourceRate >= 1000 ? `${(sourceRate / 1000).toFixed(1)}kHz` : `${sourceRate}Hz`} → ${outputRate >= 1000 ? `${(outputRate / 1000).toFixed(1)}kHz` : `${outputRate}Hz`}`
      : undefined,
  })

  return nodes
}

// ─── Quality Tier Styling ───────────────────────────────────────

function tierStyles(tier: QualityTier, active: boolean, bypassed?: boolean) {
  if (bypassed) {
    return {
      bg: 'bg-[rgb(var(--surface-raised))]',
      border: 'border-[rgb(var(--border-subtle))]',
      text: 'text-[rgb(var(--text-muted))]',
      labelText: 'text-[rgb(var(--text-muted))]',
    }
  }
  if (!active) {
    return {
      bg: 'bg-[rgb(var(--surface-sunken))]',
      border: 'border-[rgb(var(--border-subtle))]',
      text: 'text-[rgb(var(--text-muted))]',
      labelText: 'text-[rgb(var(--text-muted))]',
    }
  }
  const map: Record<QualityTier, { bg: string; border: string; text: string; labelText: string }> = {
    enhanced: {
      bg: 'bg-[rgba(var(--quality-enhanced-bg))]',
      border: 'border-[rgba(var(--quality-enhanced-border))]',
      text: 'text-[rgb(var(--quality-enhanced))]',
      labelText: 'text-[rgb(var(--quality-enhanced))]',
    },
    lossless: {
      bg: 'bg-[rgba(var(--quality-lossless-bg))]',
      border: 'border-[rgba(var(--quality-lossless-border))]',
      text: 'text-[rgb(var(--quality-lossless))]',
      labelText: 'text-[rgb(var(--quality-lossless))]',
    },
    high: {
      bg: 'bg-[rgba(var(--quality-high-bg))]',
      border: 'border-[rgba(var(--quality-high-border))]',
      text: 'text-[rgb(var(--quality-high))]',
      labelText: 'text-[rgb(var(--quality-high))]',
    },
    standard: {
      bg: 'bg-[rgba(var(--quality-standard-bg))]',
      border: 'border-[rgba(var(--quality-standard-border))]',
      text: 'text-[rgb(var(--quality-standard))]',
      labelText: 'text-[rgb(var(--quality-standard))]',
    },
    low: {
      bg: 'bg-[rgba(var(--quality-low-bg))]',
      border: 'border-[rgba(var(--quality-low-border))]',
      text: 'text-[rgb(var(--quality-low))]',
      labelText: 'text-[rgb(var(--quality-low))]',
    },
  }
  return map[tier]
}

function arrowColor(tier: QualityTier): string {
  const map: Record<QualityTier, string> = {
    enhanced: 'text-[rgb(var(--quality-enhanced))]',
    lossless: 'text-[rgb(var(--quality-lossless))]',
    high: 'text-[rgb(var(--quality-high))]',
    standard: 'text-[rgb(var(--quality-standard))]',
    low: 'text-[rgb(var(--quality-low))]',
  }
  return map[tier]
}

// ─── Visual Components ──────────────────────────────────────────

interface NodeChipProps {
  node: PipelineNode
  expanded: boolean
  onClick: () => void
}

function NodeChip({ node, expanded, onClick }: NodeChipProps) {
  const styles = tierStyles(node.tier, node.active, node.bypassed)

  return (
    <button
      onClick={onClick}
      className={`group relative flex flex-col px-2.5 py-1.5 rounded-md text-xs font-mono border transition-all
        ${styles.bg} ${styles.border} ${styles.text}
        hover:brightness-110 cursor-pointer select-none`}
      title={node.warning ?? (node.bypassed ? `${node.label} (bypassed)` : TIER_LABEL[node.tier])}
    >
      {/* Top: label + detail */}
      <span className="leading-tight whitespace-nowrap">
        {node.label}
        {node.detail && (
          <span className="opacity-60 ml-1">{node.detail}</span>
        )}
        {node.bypassed && (
          <span className="opacity-40 ml-1">(off)</span>
        )}
      </span>

      {/* Bottom: quality tier label */}
      {node.active && !node.bypassed && (
        <span className={`text-[10px] leading-tight mt-0.5 ${styles.labelText} opacity-70`}>
          {TIER_LABEL[node.tier]}
        </span>
      )}

      {/* Warning indicator */}
      {node.warning && (
        <span className="absolute -top-1 -right-1 w-3 h-3 flex items-center justify-center rounded-full bg-[rgb(var(--quality-standard))] text-[8px] text-black font-bold">
          !
        </span>
      )}

      {/* Expanded indicator */}
      {expanded && (
        <div className="absolute -bottom-0.5 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full bg-current opacity-60" />
      )}
    </button>
  )
}

function Arrow({ tier }: { tier: QualityTier }) {
  return (
    <svg
      className={`shrink-0 opacity-60 ${arrowColor(tier)}`}
      width="14"
      height="10"
      viewBox="0 0 14 10"
      fill="none"
      aria-hidden="true"
    >
      <path
        d="M0 5H11M11 5L7 1M11 5L7 9"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )
}

// ─── Node Detail Panel ──────────────────────────────────────────

function NodeDetail({ node, pipelineState }: {
  node: PipelineNode
  pipelineState: ReturnType<ReturnType<typeof useWebAudioPlayer>['getPipelineState']>
}) {
  const styles = tierStyles(node.tier, node.active, node.bypassed)

  return (
    <div className={`mt-2 p-3 rounded-lg border text-xs ${styles.bg} ${styles.border} animate-[fadeIn_var(--duration-normal)_ease-out]`}>
      <div className="flex items-center justify-between mb-2">
        <span className={`font-medium ${styles.text}`}>{node.label}</span>
        <span className={`${styles.labelText} opacity-70`}>{TIER_LABEL[node.tier]}</span>
      </div>

      {node.id === 'source' && pipelineState && (
        <div className="space-y-1 text-[rgb(var(--text-secondary))]">
          <Row label="Codec" value={pipelineState.inputFormat.codec.toUpperCase()} />
          <Row label="Sample Rate" value={`${pipelineState.inputFormat.sampleRate >= 1000 ? `${(pipelineState.inputFormat.sampleRate / 1000).toFixed(1)} kHz` : `${pipelineState.inputFormat.sampleRate} Hz`}`} />
          <Row label="Bit Depth" value={`${pipelineState.inputFormat.bitDepth}-bit`} />
          <Row label="Channels" value={`${pipelineState.inputFormat.channels}`} />
        </div>
      )}

      {node.id === 'output' && pipelineState && (
        <div className="space-y-1 text-[rgb(var(--text-secondary))]">
          <Row label="Device Rate" value={`${pipelineState.outputDevice.sampleRate >= 1000 ? `${(pipelineState.outputDevice.sampleRate / 1000).toFixed(1)} kHz` : `${pipelineState.outputDevice.sampleRate} Hz`}`} />
          <Row label="Channels" value={`${pipelineState.outputDevice.channels}`} />
          <Row label="Latency" value={`${(pipelineState.latency * 1000).toFixed(1)} ms`} />
          {node.warning && (
            <p className="text-[rgb(var(--quality-standard))] mt-1">{node.warning}</p>
          )}
        </div>
      )}

      {node.id === 'eq' && (
        <p className="text-[rgb(var(--text-tertiary))]">
          {node.active ? '10-band parametric EQ active. EQ on a lossless source is intentional enhancement — the signal is being shaped toward your preference.' : 'Bypassed — signal passes through unmodified.'}
        </p>
      )}

      {node.id === 'compressor' && (
        <p className="text-[rgb(var(--text-tertiary))]">
          {node.active ? 'Dynamic range compression active. Reduces peak-to-average ratio without altering frequency content.' : 'Bypassed — dynamics unaltered.'}
        </p>
      )}

      {node.id === 'replaygain' && (
        <p className="text-[rgb(var(--text-tertiary))]">
          {node.active ? 'Loudness normalization active. Per-track gain adjustment in the float domain — mathematically lossless.' : 'Off — playback at original mastered level.'}
        </p>
      )}

      {node.id === 'limiter' && (
        <p className="text-[rgb(var(--text-tertiary))]">
          {node.active ? 'Brick-wall limiter at -1 dBFS. Prevents clipping when ReplayGain boosts level.' : 'Bypassed.'}
        </p>
      )}

      {node.id === 'crossfade' && (
        <p className="text-[rgb(var(--text-tertiary))]">
          Dual-source crossfade active. Two audio sources overlap during transitions — quality is preserved for both streams.
        </p>
      )}

      {node.id === 'volume' && (
        <p className="text-[rgb(var(--text-tertiary))]">
          Master volume. Gain applied in the float domain before output — mathematically lossless.
        </p>
      )}

      {node.id === 'decode' && (
        <p className="text-[rgb(var(--text-tertiary))]">
          Browser decodes the audio stream via Web Audio API. For lossless formats, this is a bit-perfect decode.
        </p>
      )}
    </div>
  )
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between">
      <span className="text-[rgb(var(--text-tertiary))]">{label}</span>
      <span className="tabular-nums">{value}</span>
    </div>
  )
}

// ─── Quality Summary Badge ──────────────────────────────────────

export function QualityDot({ tier, className = '' }: { tier: QualityTier; className?: string }) {
  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${className}`}
      style={{ backgroundColor: `rgb(var(--quality-${tier}))` }}
      title={TIER_LABEL[tier]}
      aria-label={`Signal quality: ${TIER_LABEL[tier]}`}
    />
  )
}

// ─── Main Component ─────────────────────────────────────────────

export function SignalPath() {
  const currentTrack = usePlayerStore((s) => s.currentTrack)
  const { enabled: eqEnabled } = useEqStore()
  const { enabled: compressorEnabled } = useCompressorStore()
  const rgMode = useReplayGainStore((s) => s.mode)
  const limiterEnabled = useReplayGainStore((s) => s.limiterEnabled)
  const crossfadeMode = useMetaxisStore((s) => s.mode)
  const { getPipelineState } = useWebAudioPlayer()

  const [expandedNode, setExpandedNode] = useState<string | null>(null)

  const pipelineState = getPipelineState()
  const outputRate = pipelineState?.outputDevice.sampleRate ?? null

  const nodes = buildPipeline(
    currentTrack,
    eqEnabled,
    compressorEnabled,
    rgMode,
    limiterEnabled,
    crossfadeMode,
    outputRate,
  )

  const overallTier = nodes[nodes.length - 1]?.tier ?? 'standard'

  return (
    <div>
      {/* Overall quality summary */}
      <div className="flex items-center gap-2 mb-2">
        <QualityDot tier={overallTier} />
        <span className="text-xs font-medium" style={{ color: `rgb(var(--quality-${overallTier}))` }}>
          {TIER_LABEL[overallTier]}
        </span>
        <span className="text-xs text-[rgb(var(--text-muted))]">
          Signal Path
        </span>
      </div>

      {/* Pipeline chain */}
      <div className="flex items-start gap-1 overflow-x-auto py-1 scrollbar-thin">
        {nodes.map((node, i) => (
          <div key={node.id} className="flex items-center gap-1">
            {i > 0 && <Arrow tier={node.tier} />}
            <NodeChip
              node={node}
              expanded={expandedNode === node.id}
              onClick={() => setExpandedNode(expandedNode === node.id ? null : node.id)}
            />
          </div>
        ))}
      </div>

      {/* Expanded detail panel */}
      {expandedNode && (
        <NodeDetail
          node={nodes.find(n => n.id === expandedNode)!}
          pipelineState={pipelineState}
        />
      )}
    </div>
  )
}
