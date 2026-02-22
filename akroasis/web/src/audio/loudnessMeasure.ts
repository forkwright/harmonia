// Simplified ITU-R BS.1770-4 loudness measurement
// K-weighting approximation → mean square → integrated loudness (LUFS)

export interface LoudnessResult {
  integratedLufs: number
  peakLinear: number
}

// Pre-emphasis filter coefficients (simplified K-weighting stage 1: high shelf +4dB at 1681Hz)
// Stage 2 (high-pass at 38Hz) is omitted — negligible for music above 100Hz
const K_WEIGHT_GAIN_DB = 4
const K_WEIGHT_GAIN_LINEAR = Math.pow(10, K_WEIGHT_GAIN_DB / 20)

export function measureLoudness(buffer: AudioBuffer): LoudnessResult {
  const numChannels = buffer.numberOfChannels
  const sampleRate = buffer.sampleRate
  const length = buffer.length

  // Block size: 400ms with 75% overlap (100ms hop)
  const blockSize = Math.round(sampleRate * 0.4)
  const hopSize = Math.round(sampleRate * 0.1)

  // Channel weights per BS.1770: L=R=1.0, C=1.0, Ls=Rs=1.41
  // For stereo (most common): both channels = 1.0
  const channelWeights = new Float64Array(numChannels)
  for (let ch = 0; ch < numChannels; ch++) {
    channelWeights[ch] = (ch >= 4) ? 1.41 : 1.0
  }

  // Get channel data
  const channels: Float32Array[] = []
  let peakLinear = 0
  for (let ch = 0; ch < numChannels; ch++) {
    const data = buffer.getChannelData(ch)
    channels.push(data)
    for (let i = 0; i < length; i++) {
      const abs = Math.abs(data[i])
      if (abs > peakLinear) peakLinear = abs
    }
  }

  // Compute block loudness values
  const blockLoudness: number[] = []
  for (let start = 0; start + blockSize <= length; start += hopSize) {
    let blockPower = 0
    for (let ch = 0; ch < numChannels; ch++) {
      const data = channels[ch]
      let channelMeanSquare = 0
      for (let i = start; i < start + blockSize; i++) {
        // Apply simplified K-weighting (high shelf boost)
        const sample = data[i] * K_WEIGHT_GAIN_LINEAR
        channelMeanSquare += sample * sample
      }
      channelMeanSquare /= blockSize
      blockPower += channelWeights[ch] * channelMeanSquare
    }
    blockLoudness.push(blockPower)
  }

  if (blockLoudness.length === 0) {
    return { integratedLufs: -70, peakLinear }
  }

  // Absolute gating: -70 LUFS threshold
  const absoluteThreshold = Math.pow(10, -7) // -70 LUFS in linear power
  const aboveAbsolute = blockLoudness.filter((p) => p > absoluteThreshold)

  if (aboveAbsolute.length === 0) {
    return { integratedLufs: -70, peakLinear }
  }

  // Relative gating: mean of blocks above absolute threshold, then -10 LU
  const meanAbsolute = aboveAbsolute.reduce((a, b) => a + b, 0) / aboveAbsolute.length
  const relativeThreshold = meanAbsolute * Math.pow(10, -1) // -10 LU

  const aboveRelative = aboveAbsolute.filter((p) => p > relativeThreshold)

  if (aboveRelative.length === 0) {
    return { integratedLufs: -70, peakLinear }
  }

  // Integrated loudness = mean of blocks above relative threshold
  const meanPower = aboveRelative.reduce((a, b) => a + b, 0) / aboveRelative.length
  const integratedLufs = -0.691 + 10 * Math.log10(meanPower)

  return {
    integratedLufs: Number.isFinite(integratedLufs) ? integratedLufs : -70,
    peakLinear,
  }
}

// Compute gain adjustment to reach target loudness
export function computeReplayGain(measuredLufs: number, targetLufs: number): number {
  return targetLufs - measuredLufs
}

// Convert dB to linear gain multiplier
export function dbToLinear(db: number): number {
  return Math.pow(10, db / 20)
}
