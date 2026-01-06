// Settings and preferences page
import { useState } from 'react'
import { usePlayerStore } from '../stores/playerStore'
import { Card } from '../components/Card'
import { Button } from '../components/Button'

export function SettingsPage() {
  const { playbackSpeed, setPlaybackSpeed, volume, setVolume } = usePlayerStore()
  const [tempSpeed, setTempSpeed] = useState(playbackSpeed)
  const [tempVolume, setTempVolume] = useState(volume)

  const speedPresets = [
    { label: '0.5x', value: 0.5 },
    { label: '0.75x', value: 0.75 },
    { label: '1x', value: 1 },
    { label: 1.25, value: 1.25 },
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
      <h1 className="text-3xl font-bold mb-6">Settings</h1>

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
              <span className="font-medium">{globalThis.AudioContext ? new AudioContext().sampleRate : 48000} Hz</span>
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
