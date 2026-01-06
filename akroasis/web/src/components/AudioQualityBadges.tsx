// Audio quality indicator badges
interface AudioQualityBadgesProps {
  readonly format?: string
  readonly sampleRate?: number
  readonly bitDepth?: number
  readonly lossless?: boolean
}

export function AudioQualityBadges({
  format,
  sampleRate,
  bitDepth,
  lossless
}: AudioQualityBadgesProps) {
  const isHiRes = (sampleRate ?? 0) > 48000 || (bitDepth ?? 0) > 16
  const is24Bit = (bitDepth ?? 0) >= 24

  return (
    <div className="flex flex-wrap gap-2 justify-center mt-3">
      {format && (
        <span className="px-2 py-1 text-xs font-medium rounded bg-blue-900/30 text-blue-300 border border-blue-700/30">
          {format.toUpperCase()}
        </span>
      )}

      {isHiRes && (
        <span className="px-2 py-1 text-xs font-medium rounded bg-purple-900/30 text-purple-300 border border-purple-700/30">
          Hi-Res {sampleRate && sampleRate >= 1000 ? `${(sampleRate / 1000).toFixed(1)}kHz` : `${sampleRate}Hz`}
        </span>
      )}

      {is24Bit && (
        <span className="px-2 py-1 text-xs font-medium rounded bg-green-900/30 text-green-300 border border-green-700/30">
          24-bit
        </span>
      )}

      {lossless && (
        <span className="px-2 py-1 text-xs font-medium rounded bg-amber-900/30 text-amber-300 border border-amber-700/30">
          Lossless
        </span>
      )}

      <span className="px-2 py-1 text-xs font-medium rounded bg-yellow-900/30 text-yellow-300 border border-yellow-700/30">
        Browser Resampling
      </span>
    </div>
  )
}
