// Audio quality badges using the quality tier system
import { getSourceTier, type QualityTier } from './SignalPath'

const TIER_LABEL: Record<QualityTier, string> = {
  enhanced: 'Hi-Res',
  lossless: 'Lossless',
  high: 'High Quality',
  standard: 'Standard',
  low: 'Low Quality',
}

interface AudioQualityBadgesProps {
  readonly format?: string
  readonly sampleRate?: number
  readonly bitDepth?: number
}

export function AudioQualityBadges({ format, sampleRate, bitDepth }: AudioQualityBadgesProps) {
  const tier = getSourceTier({ format, sampleRate, bitDepth })
  const isHiRes = (sampleRate ?? 0) > 48000 || (bitDepth ?? 0) > 16

  return (
    <div className="flex flex-wrap gap-2 justify-center mt-3">
      {/* Format badge colored by quality tier */}
      {format && (
        <span
          className="px-2 py-1 text-xs font-medium rounded border"
          style={{
            backgroundColor: `rgba(var(--quality-${tier}-bg))`,
            borderColor: `rgba(var(--quality-${tier}-border))`,
            color: `rgb(var(--quality-${tier}))`,
          }}
        >
          {format.toUpperCase()}
        </span>
      )}

      {/* Hi-Res detail */}
      {isHiRes && sampleRate && (
        <span
          className="px-2 py-1 text-xs font-medium rounded border"
          style={{
            backgroundColor: 'rgba(var(--quality-enhanced-bg))',
            borderColor: 'rgba(var(--quality-enhanced-border))',
            color: 'rgb(var(--quality-enhanced))',
          }}
        >
          {sampleRate >= 1000 ? `${(sampleRate / 1000).toFixed(1)}kHz` : `${sampleRate}Hz`}
          {bitDepth ? `/${bitDepth}bit` : ''}
        </span>
      )}

      {/* Quality tier label */}
      <span
        className="px-2 py-1 text-xs font-medium rounded border"
        style={{
          backgroundColor: `rgba(var(--quality-${tier}-bg))`,
          borderColor: `rgba(var(--quality-${tier}-border))`,
          color: `rgb(var(--quality-${tier}))`,
        }}
      >
        {TIER_LABEL[tier]}
      </span>

      {/* Browser resampling warning — always present on web */}
      <span
        className="px-2 py-1 text-xs font-medium rounded border"
        style={{
          backgroundColor: 'rgba(var(--quality-standard-bg))',
          borderColor: 'rgba(var(--quality-standard-border))',
          color: 'rgb(var(--quality-standard))',
        }}
      >
        Browser Resampling
      </span>
    </div>
  )
}
