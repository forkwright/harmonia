import '@testing-library/jest-dom'
import { afterEach, vi } from 'vitest'
import { cleanup } from '@testing-library/react'

// Cleanup after each test
afterEach(() => {
  cleanup()
})

// Mock Web Audio API (not available in test environment)
globalThis.AudioContext = class MockAudioContext {
  createGain = vi.fn(() => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    gain: { value: 1, setValueAtTime: vi.fn() },
  }))

  createBufferSource = vi.fn(() => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    start: vi.fn(),
    stop: vi.fn(),
    buffer: null,
    onended: null,
    playbackRate: { value: 1 },
  }))

  createBiquadFilter = vi.fn(() => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    type: 'peaking' as BiquadFilterType,
    frequency: { value: 0 },
    Q: { value: 0 },
    gain: { value: 0 },
  }))

  createAnalyser = vi.fn(() => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    fftSize: 2048,
    getByteTimeDomainData: vi.fn(),
    getByteFrequencyData: vi.fn(),
    frequencyBinCount: 1024,
  }))

  createDynamicsCompressor = vi.fn(() => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    threshold: { value: -24, setValueAtTime: vi.fn() },
    knee: { value: 30, setValueAtTime: vi.fn() },
    ratio: { value: 12, setValueAtTime: vi.fn() },
    attack: { value: 0.003, setValueAtTime: vi.fn() },
    release: { value: 0.25, setValueAtTime: vi.fn() },
  }))

  decodeAudioData = vi.fn(() => Promise.resolve({
    duration: 180,
    length: 7938000,
    numberOfChannels: 2,
    sampleRate: 44100,
  }))

  destination = { maxChannelCount: 2 }
  currentTime = 0
  sampleRate = 44100
  state = 'running'
  baseLatency = 0

  suspend = vi.fn(() => Promise.resolve())
  resume = vi.fn(() => Promise.resolve())
  close = vi.fn(() => Promise.resolve())
} as unknown as typeof AudioContext

// Mock navigator.mediaSession
Object.defineProperty(globalThis.navigator, 'mediaSession', {
  value: {
    metadata: null,
    playbackState: 'none',
    setActionHandler: vi.fn(),
    setPositionState: vi.fn(),
    setMicrophoneActive: vi.fn(),
    setCameraActive: vi.fn(),
  },
  writable: true,
  configurable: true,
})
