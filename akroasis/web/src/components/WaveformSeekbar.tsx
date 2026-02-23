// Canvas-based waveform seekbar with live analyser data
import { useRef, useEffect, useCallback } from 'react'

interface WaveformSeekbarProps {
  analyserNode: AnalyserNode | null
  duration: number // ms
  position: number // ms
  onSeek: (ms: number) => void
  disabled?: boolean
}

const HEIGHT = 48

function getCSSColor(varName: string): string {
  const raw = getComputedStyle(document.documentElement).getPropertyValue(varName).trim()
  // Handle "r g b" or "r g b / a" format
  if (raw.includes('/')) {
    const [rgb, a] = raw.split('/')
    return `rgba(${rgb.trim().replace(/ /g, ', ')}, ${a.trim()})`
  }
  return `rgb(${raw.replace(/ /g, ', ')})`
}

function getThemeColors() {
  return {
    played: getCSSColor('--accent-primary'),
    unplayed: getCSSColor('--border-subtle'),
    position: getCSSColor('--accent-hover'),
    empty: getCSSColor('--surface-sunken'),
  }
}

export function WaveformSeekbar({ analyserNode, duration, position, onSeek, disabled }: WaveformSeekbarProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const animRef = useRef<number>(0)
  const dataRef = useRef<Uint8Array<ArrayBuffer> | null>(null)
  const drawRef = useRef<() => void>(() => {})

  const draw = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const dpr = globalThis.devicePixelRatio || 1
    const w = canvas.clientWidth
    const h = canvas.clientHeight

    if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
      canvas.width = w * dpr
      canvas.height = h * dpr
      ctx.scale(dpr, dpr)
    }

    ctx.clearRect(0, 0, w, h)

    const progress = duration > 0 ? Math.min(position / duration, 1) : 0
    const splitX = progress * w

    const colors = getThemeColors()

    if (!analyserNode || !duration) {
      // Empty state — flat line
      ctx.fillStyle = colors.empty
      ctx.fillRect(0, 0, w, h)
      const midY = h / 2
      ctx.strokeStyle = colors.unplayed
      ctx.lineWidth = 1
      ctx.beginPath()
      ctx.moveTo(0, midY)
      ctx.lineTo(w, midY)
      ctx.stroke()
      return
    }

    // Get waveform data
    if (!dataRef.current || dataRef.current.length !== analyserNode.frequencyBinCount) {
      dataRef.current = new Uint8Array(analyserNode.frequencyBinCount)
    }
    analyserNode.getByteTimeDomainData(dataRef.current)

    const data = dataRef.current
    const barCount = Math.min(data.length, w)
    const step = data.length / barCount
    const barW = w / barCount

    for (let i = 0; i < barCount; i++) {
      const idx = Math.floor(i * step)
      const sample = (data[idx] - 128) / 128 // -1 to 1
      const barH = Math.max(2, Math.abs(sample) * h * 0.9)
      const x = i * barW
      const y = (h - barH) / 2

      ctx.fillStyle = x < splitX ? colors.played : colors.unplayed
      ctx.fillRect(x, y, Math.max(1, barW - 0.5), barH)
    }

    // Position indicator
    if (splitX > 0 && splitX < w) {
      ctx.fillStyle = colors.position
      ctx.fillRect(splitX - 1, 0, 2, h)
    }
  }, [analyserNode, duration, position])

  // Keep ref in sync with latest draw function
  useEffect(() => {
    drawRef.current = draw
  })

  // Animation loop — stable deps, uses ref to always call latest draw
  useEffect(() => {
    let running = true
    function tick() {
      if (!running) return
      drawRef.current()
      animRef.current = requestAnimationFrame(tick)
    }
    tick()
    return () => {
      running = false
      cancelAnimationFrame(animRef.current)
    }
  }, [])

  const handleClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (disabled || !duration) return
    const canvas = canvasRef.current
    if (!canvas) return

    const rect = canvas.getBoundingClientRect()
    const x = e.clientX - rect.left
    const ratio = Math.max(0, Math.min(1, x / rect.width))
    onSeek(ratio * duration)
  }, [disabled, duration, onSeek])

  return (
    <canvas
      ref={canvasRef}
      onClick={handleClick}
      className={`w-full rounded-lg ${disabled ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer'}`}
      style={{ height: HEIGHT }}
      aria-label="Waveform seekbar"
      role="slider"
      aria-valuemin={0}
      aria-valuemax={duration}
      aria-valuenow={position}
    />
  )
}
