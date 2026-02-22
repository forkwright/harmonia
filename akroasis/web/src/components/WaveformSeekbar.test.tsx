import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { WaveformSeekbar } from './WaveformSeekbar'

function mockAnalyser(binCount = 1024): AnalyserNode {
  return {
    frequencyBinCount: binCount,
    fftSize: 2048,
    getByteTimeDomainData: vi.fn((arr: Uint8Array) => {
      for (let i = 0; i < arr.length; i++) arr[i] = 128
    }),
    getByteFrequencyData: vi.fn(),
    connect: vi.fn(),
    disconnect: vi.fn(),
  } as unknown as AnalyserNode
}

// Canvas mock (jsdom doesn't have real canvas)
beforeEach(() => {
  HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
    clearRect: vi.fn(),
    fillRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    scale: vi.fn(),
    fillStyle: '',
    strokeStyle: '',
    lineWidth: 1,
  })) as unknown as typeof HTMLCanvasElement.prototype.getContext
})

describe('WaveformSeekbar', () => {
  it('renders a canvas element', () => {
    render(<WaveformSeekbar analyserNode={null} duration={0} position={0} onSeek={vi.fn()} />)
    expect(screen.getByRole('slider')).toBeInTheDocument()
  })

  it('has correct aria attributes', () => {
    render(<WaveformSeekbar analyserNode={null} duration={60000} position={15000} onSeek={vi.fn()} />)
    const slider = screen.getByRole('slider')
    expect(slider).toHaveAttribute('aria-valuemin', '0')
    expect(slider).toHaveAttribute('aria-valuemax', '60000')
    expect(slider).toHaveAttribute('aria-valuenow', '15000')
  })

  it('applies disabled styling', () => {
    render(<WaveformSeekbar analyserNode={null} duration={60000} position={0} onSeek={vi.fn()} disabled />)
    const canvas = screen.getByRole('slider')
    expect(canvas.className).toContain('opacity-40')
    expect(canvas.className).toContain('cursor-not-allowed')
  })

  it('applies pointer cursor when enabled', () => {
    render(<WaveformSeekbar analyserNode={mockAnalyser()} duration={60000} position={0} onSeek={vi.fn()} />)
    const canvas = screen.getByRole('slider')
    expect(canvas.className).toContain('cursor-pointer')
    expect(canvas.className).not.toContain('cursor-not-allowed')
  })

  it('calls onSeek with correct position on click', () => {
    const onSeek = vi.fn()
    render(<WaveformSeekbar analyserNode={mockAnalyser()} duration={60000} position={0} onSeek={onSeek} />)
    const canvas = screen.getByRole('slider')

    // Mock getBoundingClientRect
    vi.spyOn(canvas, 'getBoundingClientRect').mockReturnValue({
      left: 0, right: 200, top: 0, bottom: 48, width: 200, height: 48, x: 0, y: 0, toJSON: vi.fn(),
    })

    fireEvent.click(canvas, { clientX: 100, clientY: 24 })
    expect(onSeek).toHaveBeenCalledWith(30000) // 100/200 * 60000
  })

  it('does not call onSeek when disabled', () => {
    const onSeek = vi.fn()
    render(<WaveformSeekbar analyserNode={mockAnalyser()} duration={60000} position={0} onSeek={onSeek} disabled />)
    const canvas = screen.getByRole('slider')
    fireEvent.click(canvas, { clientX: 100, clientY: 24 })
    expect(onSeek).not.toHaveBeenCalled()
  })

  it('does not call onSeek when duration is 0', () => {
    const onSeek = vi.fn()
    render(<WaveformSeekbar analyserNode={mockAnalyser()} duration={0} position={0} onSeek={onSeek} />)
    const canvas = screen.getByRole('slider')
    fireEvent.click(canvas, { clientX: 100, clientY: 24 })
    expect(onSeek).not.toHaveBeenCalled()
  })

  it('reads analyser data when node is provided', () => {
    const analyser = mockAnalyser()
    render(<WaveformSeekbar analyserNode={analyser} duration={60000} position={30000} onSeek={vi.fn()} />)
    // getByteTimeDomainData is called via requestAnimationFrame — just verify it was invoked
    expect(analyser.getByteTimeDomainData).toHaveBeenCalled()
  })

  it('clamps seek to 0–duration range', () => {
    const onSeek = vi.fn()
    render(<WaveformSeekbar analyserNode={mockAnalyser()} duration={60000} position={0} onSeek={onSeek} />)
    const canvas = screen.getByRole('slider')

    vi.spyOn(canvas, 'getBoundingClientRect').mockReturnValue({
      left: 100, right: 300, top: 0, bottom: 48, width: 200, height: 48, x: 100, y: 0, toJSON: vi.fn(),
    })

    // Click before canvas start (negative offset)
    fireEvent.click(canvas, { clientX: 50, clientY: 24 })
    expect(onSeek).toHaveBeenCalledWith(0) // clamped to 0

    // Click after canvas end
    fireEvent.click(canvas, { clientX: 400, clientY: 24 })
    expect(onSeek).toHaveBeenCalledWith(60000) // clamped to max
  })
})
