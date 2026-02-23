// Signal path visualization — shows the real audio chain
// Reads actual track metadata (format, sample rate, bit depth, bitrate)
// and live playback state (buffered %, network state)
import { useState, useEffect } from 'react'
import { usePlayerStore } from '../stores/playerStore'
import { useWebAudioPlayer } from '../hooks/useWebAudioPlayer'

// ─── Quality Tier System ────────────────────────────────────────

export type QualityTier = 'enhanced' | 'lossless' | 'high' | 'standard' | 'low'

const TIER_LABEL: Record<QualityTier, string> = {
  enhanced: 'Enhanced',
  lossless: 'Lossless',
  high: 'High Quality',
  standard: 'Standard',
  low: 'Low Quality',
}

export function getSourceTier(track: {
  format?: string
  sampleRate?: number
  bitDepth?: number
  bitrate?: number
} | null): QualityTier {
  if (!track) return 'standard'

  const fmt = (track.format || '').toLowerCase()
  const sr = track.sampleRate ?? 0
  const bd = track.bitDepth ?? 0
  const isLossless = ['flac', 'wav', 'alac', 'aiff', 'dsd', 'dsf', 'dff', 'wv', 'ape'].includes(fmt)

  if (isLossless && (sr > 48000 || bd > 16)) return 'enhanced'
  if (isLossless) return 'lossless'
  if (['aac', 'ogg', 'opus'].includes(fmt)) return 'high'
  if (fmt === 'mp3' && (track.bitrate ?? 0) >= 256) return 'high'
  if (fmt) return 'standard'
  return 'standard'
}

// ─── Formatting ─────────────────────────────────────────────────

function formatSampleRate(sr: number): string {
  if (sr >= 1000) return `${(sr / 1000).toFixed(sr % 1000 === 0 ? 0 : 1)}kHz`
  return `${sr}Hz`
}

function formatBitrate(br: number): string {
  if (br >= 1000) return `${(br / 1000).toFixed(0)} Mbps`
  return `${br} kbps`
}

function formatFileSize(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`
  if (bytes >= 1_000) return `${(bytes / 1_000).toFixed(0)} KB`
  return `${bytes} B`
}

function channelLabel(ch: number): string {
  if (ch === 1) return 'Mono'
  if (ch === 2) return 'Stereo'
  if (ch === 6) return '5.1'
  if (ch === 8) return '7.1'
  return `${ch}ch`
}

// ─── Pipeline ───────────────────────────────────────────────────

interface PipelineNode {
  id: string
  label: string
  badge: string
  tier: QualityTier
  lines: string[]
}

function buildPipeline(
  track: {
    format?: string; sampleRate?: number; bitDepth?: number;
    channels?: number; bitrate?: number; fileSize?: number
  } | null,
  playbackInfo: { bufferedPercent: number; networkState: string; readyState: string } | null
): PipelineNode[] {
  const tier = getSourceTier(track)
  const fmt = (track?.format || '').toUpperCase()

  // Source
  const sourceLines: string[] = []
  if (fmt) sourceLines.push(fmt)
  if (track?.sampleRate) sourceLines.push(formatSampleRate(track.sampleRate))
  if (track?.bitDepth) sourceLines.push(`${track.bitDepth}-bit`)
  if (track?.channels) sourceLines.push(channelLabel(track.channels))
  if (track?.bitrate) sourceLines.push(formatBitrate(track.bitrate))
  if (track?.fileSize) sourceLines.push(formatFileSize(track.fileSize))

  // Transport
  const transportLines: string[] = []
  if (playbackInfo) {
    transportLines.push(`Buffered: ${playbackInfo.bufferedPercent}%`)
    if (playbackInfo.networkState === 'LOADING') transportLines.push('Streaming...')
    else if (playbackInfo.networkState === 'IDLE') transportLines.push('Buffered')
  }

  // Decode
  const isLossless = ['FLAC', 'WAV', 'ALAC', 'AIFF', 'DSD', 'WV', 'APE'].includes(fmt)
  const decodeLines: string[] = []
  decodeLines.push('Browser native decoder')
  if (isLossless) decodeLines.push('Bit-perfect lossless')
  else if (fmt) decodeLines.push(`${fmt} lossy decode`)

  // Output
  const outputLines = ['HTMLAudioElement → system output', 'No DSP processing']

  return [
    {
      id: 'source',
      label: 'Source',
      badge: fmt || '—',
      tier,
      lines: sourceLines.length > 0 ? sourceLines : ['Unknown format'],
    },
    {
      id: 'transport',
      label: 'Stream',
      badge: playbackInfo ? `${playbackInfo.bufferedPercent}%` : '—',
      tier,
      lines: transportLines.length > 0 ? transportLines : ['Waiting...'],
    },
    {
      id: 'decode',
      label: 'Decode',
      badge: isLossless ? 'Lossless' : 'Native',
      tier: isLossless ? tier : (tier === 'enhanced' ? 'lossless' : tier),
      lines: decodeLines,
    },
    {
      id: 'output',
      label: 'Output',
      badge: 'Direct',
      tier,
      lines: outputLines,
    },
  ]
}

// ─── Styling ────────────────────────────────────────────────────

function tierColor(tier: QualityTier): string {
  return `rgb(var(--quality-${tier}))`
}

// ─── Components ─────────────────────────────────────────────────

export function QualityDot({ tier, className = '' }: { tier: QualityTier; className?: string }) {
  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${className}`}
      style={{ backgroundColor: tierColor(tier) }}
      title={TIER_LABEL[tier]}
      aria-label={`Signal quality: ${TIER_LABEL[tier]}`}
    />
  )
}

export function SignalPath() {
  const currentTrack = usePlayerStore((s) => s.currentTrack)
  const isPlaying = usePlayerStore((s) => s.isPlaying)
  const { getPlaybackInfo } = useWebAudioPlayer()
  const [expandedNode, setExpandedNode] = useState<string | null>(null)
  const [playbackInfo, setPlaybackInfo] = useState<ReturnType<typeof getPlaybackInfo>>(null)

  // Poll playback info while playing (buffered % changes over time)
  useEffect(() => {
    if (!isPlaying) return
    const id = setInterval(() => {
      setPlaybackInfo(getPlaybackInfo())
    }, 1000)
    return () => clearInterval(id)
  }, [isPlaying, getPlaybackInfo])

  const nodes = buildPipeline(currentTrack, playbackInfo)
  const overallTier = nodes[0]?.tier ?? 'standard'

  return (
    <div>
      {/* Overall quality badge */}
      <div className="flex items-center gap-2 mb-3">
        <QualityDot tier={overallTier} />
        <span className="text-xs font-medium" style={{ color: tierColor(overallTier) }}>
          {TIER_LABEL[overallTier]}
        </span>
        <span className="text-xs" style={{ color: 'rgb(var(--text-muted))' }}>Signal Path</span>
      </div>

      {/* Node chain */}
      <div className="flex items-center gap-1 overflow-x-auto pb-1">
        {nodes.map((node, i) => (
          <div key={node.id} className="flex items-center gap-1">
            {i > 0 && (
              <svg className="shrink-0 opacity-40" width="16" height="10" viewBox="0 0 16 10" fill="none" aria-hidden>
                <path d="M0 5H13M13 5L9 1M13 5L9 9" stroke={tierColor(overallTier)} strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            )}
            <button
              onClick={() => setExpandedNode(expandedNode === node.id ? null : node.id)}
              className="flex flex-col items-start px-2.5 py-1.5 rounded-md text-xs font-mono border transition-all hover:brightness-110 select-none min-w-[64px]"
              style={{
                backgroundColor: `color-mix(in srgb, ${tierColor(node.tier)} 8%, rgb(var(--surface-raised)))`,
                borderColor: expandedNode === node.id
                  ? tierColor(node.tier)
                  : `color-mix(in srgb, ${tierColor(node.tier)} 25%, rgb(var(--border-default)))`,
                color: tierColor(node.tier),
              }}
            >
              <span className="text-[10px] uppercase tracking-wider opacity-60 leading-tight">
                {node.label}
              </span>
              <span className="font-medium leading-tight whitespace-nowrap">
                {node.badge}
              </span>
            </button>
          </div>
        ))}
      </div>

      {/* Detail panel */}
      {expandedNode && (() => {
        const node = nodes.find(n => n.id === expandedNode)
        if (!node) return null
        return (
          <div
            className="mt-2 p-3 rounded-lg border text-xs space-y-1"
            style={{
              backgroundColor: `color-mix(in srgb, ${tierColor(node.tier)} 5%, rgb(var(--surface-raised)))`,
              borderColor: `color-mix(in srgb, ${tierColor(node.tier)} 20%, rgb(var(--border-default)))`,
            }}
          >
            <div className="font-medium mb-1.5" style={{ color: tierColor(node.tier) }}>
              {node.label}
            </div>
            {node.lines.map((line, i) => (
              <div key={i} style={{ color: 'rgb(var(--text-secondary))' }}>
                {line}
              </div>
            ))}
          </div>
        )
      })()}
    </div>
  )
}
