// Settings and preferences page
import { useState, useMemo } from 'react'
import { usePlayerStore } from '../stores/playerStore'
import { useReplayGainStore } from '../stores/replayGainStore'
import { useMetaxisStore } from '../stores/metaxisStore'
import type { ReplayGainMode } from '../stores/replayGainStore'
import type { CrossfadeCurve } from '../stores/metaxisStore'
import { Card } from '../components/Card'
import { Button } from '../components/Button'

const rgModes: { label: string; value: ReplayGainMode }[] = [
  { label: 'Off', value: 'off' },
  { label: 'Track', value: 'track' },
  { label: 'Album', value: 'album' },
]

function ReplayGainSettings() {
  const { mode, targetLufs, limiterEnabled, preScanEnabled, setMode, setTargetLufs, setLimiterEnabled, setPreScanEnabled } = useReplayGainStore()

  return (
    <Card>
      <h2 className="text-xl font-semibold mb-4">ReplayGain</h2>

      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium mb-2">Mode</label>
          <div className="flex gap-2">
            {rgModes.map((opt) => (
              <Button
                key={opt.value}
                onClick={() => setMode(opt.value)}
                className={`px-3 py-1 text-sm ${
                  mode === opt.value
                    ? 'bg-blue-600 text-white'
                    : 'bg-gray-200 text-gray-800 hover:bg-gray-300'
                }`}
              >
                {opt.label}
              </Button>
            ))}
          </div>
        </div>

        {mode !== 'off' && (
          <>
            <div>
              <label className="block text-sm font-medium mb-2">
                Target Loudness: {targetLufs} LUFS
              </label>
              <input
                type="range"
                min="-23"
                max="-14"
                step="1"
                value={targetLufs}
                onChange={(e) => setTargetLufs(Number.parseInt(e.target.value))}
                className="w-full"
              />
              <div className="flex justify-between text-xs text-gray-500 mt-1">
                <span>-23 (quieter)</span>
                <span>-18 (EBU R128)</span>
                <span>-14 (louder)</span>
              </div>
            </div>

            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={limiterEnabled}
                onChange={(e) => setLimiterEnabled(e.target.checked)}
                className="rounded"
              />
              Brick-wall limiter (-1 dBFS)
            </label>

            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={preScanEnabled}
                onChange={(e) => setPreScanEnabled(e.target.checked)}
                className="rounded"
              />
              Analyze tracks without ReplayGain tags
            </label>
          </>
        )}
      </div>
    </Card>
  )
}

const cfCurves: { label: string; value: CrossfadeCurve }[] = [
  { label: 'Linear', value: 'linear' },
  { label: 'Equal Power', value: 'equalPower' },
  { label: 'S-Curve', value: 'sCurve' },
]

function CrossfadeSettings() {
  const { mode, duration, curve, respectAlbumTransitions, setMode, setDuration, setCurve, setRespectAlbumTransitions } = useMetaxisStore()

  return (
    <Card>
      <h2 className="text-xl font-semibold mb-4">Crossfade</h2>

      <div className="space-y-4">
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={mode === 'simple'}
            onChange={(e) => setMode(e.target.checked ? 'simple' : 'off')}
            className="rounded"
          />
          Enable crossfade
        </label>

        {mode !== 'off' && (
          <>
            <div>
              <label className="block text-sm font-medium mb-2">
                Duration: {duration}s
              </label>
              <input
                type="range"
                min="1"
                max="12"
                step="0.5"
                value={duration}
                onChange={(e) => setDuration(Number.parseFloat(e.target.value))}
                className="w-full"
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">Curve</label>
              <div className="flex gap-2">
                {cfCurves.map((opt) => (
                  <Button
                    key={opt.value}
                    onClick={() => setCurve(opt.value)}
                    className={`px-3 py-1 text-sm ${
                      curve === opt.value
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-200 text-gray-800 hover:bg-gray-300'
                    }`}
                  >
                    {opt.label}
                  </Button>
                ))}
              </div>
            </div>

            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={respectAlbumTransitions}
                onChange={(e) => setRespectAlbumTransitions(e.target.checked)}
                className="rounded"
              />
              Skip crossfade for consecutive album tracks
            </label>
          </>
        )}
      </div>
    </Card>
  )
}

export function SettingsPage() {
  const { playbackSpeed, setPlaybackSpeed, volume, setVolume } = usePlayerStore()
  const [tempSpeed, setTempSpeed] = useState(playbackSpeed)
  const [tempVolume, setTempVolume] = useState(volume)

  const sampleRate = useMemo(() => {
    if (!globalThis.AudioContext) return 48000
    const ctx = new AudioContext()
    const rate = ctx.sampleRate
    void ctx.close()
    return rate
  }, [])

  const speedPresets = [
    { label: '0.5x', value: 0.5 },
    { label: '0.75x', value: 0.75 },
    { label: '1x', value: 1 },
    { label: '1.25x', value: 1.25 },
    { label: '1.5x', value: 1.5 },
    { label: '1.75x', value: 1.75 },
    { label: '2x', value: 2 },
  ]

  const handleSpeedChange = (speed: number) => {
    setTempSpeed(speed)
    setPlaybackSpeed(speed)
  }

  const handleVolumeChange = (vol: number) => {
    setTempVolume(vol)
    setVolume(vol)
  }

  return (
    <div className="container mx-auto p-6 max-w-4xl">
      <h1 className="text-3xl font-serif font-semibold mb-6" style={{ color: 'rgb(var(--text-primary))' }}>Settings</h1>

      <div className="space-y-6">
        {/* Playback Settings */}
        <Card>
          <h2 className="text-xl font-semibold mb-4">Playback</h2>

          <div className="space-y-4">
            {/* Playback Speed */}
            <div>
              <label className="block text-sm font-medium mb-2">
                Playback Speed: {tempSpeed.toFixed(2)}x
              </label>
              <div className="flex items-center gap-4">
                <input
                  type="range"
                  min="0.5"
                  max="2"
                  step="0.05"
                  value={tempSpeed}
                  onChange={(e) => handleSpeedChange(Number.parseFloat(e.target.value))}
                  className="flex-1"
                />
              </div>
              <div className="flex gap-2 mt-2 flex-wrap">
                {speedPresets.map((preset) => (
                  <Button
                    key={preset.value}
                    onClick={() => handleSpeedChange(preset.value)}
                    className={`px-3 py-1 text-sm ${
                      Math.abs(tempSpeed - preset.value) < 0.01
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-200 text-gray-800 hover:bg-gray-300'
                    }`}
                  >
                    {preset.label}
                  </Button>
                ))}
              </div>
            </div>

            {/* Volume */}
            <div>
              <label className="block text-sm font-medium mb-2">
                Volume: {Math.round(tempVolume * 100)}%
              </label>
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={tempVolume}
                onChange={(e) => handleVolumeChange(Number.parseFloat(e.target.value))}
                className="w-full"
              />
            </div>
          </div>
        </Card>

        {/* ReplayGain */}
        <ReplayGainSettings />

        {/* Crossfade (Metaxis) */}
        <CrossfadeSettings />

        {/* Audio Quality Settings */}
        <Card>
          <h2 className="text-xl font-semibold mb-4">Audio Quality</h2>

          <div className="space-y-3 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-600">Browser Audio Stack</span>
              <span className="font-medium">Web Audio API</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Sample Rate</span>
              <span className="font-medium">{sampleRate} Hz</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Bit-Perfect Playback</span>
              <span className="font-medium text-yellow-600">Not Available (Browser Limitation)</span>
            </div>
            <p className="text-xs text-gray-500 mt-4">
              Note: Web browsers resample all audio to the system sample rate. For bit-perfect playback, use the Android app.
            </p>
          </div>
        </Card>

        {/* About */}
        <Card>
          <h2 className="text-xl font-semibold mb-4">About</h2>

          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-600">Version</span>
              <span className="font-medium">Web MVP v1.0</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Platform</span>
              <span className="font-medium">React 19 + Vite + TypeScript</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600">Audio Engine</span>
              <span className="font-medium">Web Audio API</span>
            </div>
            <p className="text-xs text-gray-500 mt-4">
              Akroasis - Enhanced media player with bit-perfect audio support (Android)
            </p>
          </div>
        </Card>
      </div>
    </div>
  )
}
