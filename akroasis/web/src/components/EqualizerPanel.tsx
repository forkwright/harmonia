// 10-band parametric EQ panel with preset chips, enable toggle, and band sliders
import { useState } from 'react'
import { useEqStore, BUILT_IN_PRESETS } from '../stores/eqStore'

const FREQUENCIES = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]

function formatFreq(hz: number): string {
  return hz >= 1000 ? `${hz / 1000}k` : `${hz}`
}

export function EqualizerPanel() {
  const {
    enabled,
    bands,
    activePreset,
    customPresets,
    setBand,
    setPreset,
    saveCustomPreset,
    deleteCustomPreset,
    setEnabled,
    reset,
  } = useEqStore()

  const [saving, setSaving] = useState(false)
  const [presetName, setPresetName] = useState('')

  const builtInNames = Object.keys(BUILT_IN_PRESETS)
  const customNames = Object.keys(customPresets)

  function handleSave() {
    if (!saving) {
      setSaving(true)
      return
    }
    const name = presetName.trim()
    if (name) {
      saveCustomPreset(name)
    }
    setSaving(false)
    setPresetName('')
  }

  return (
    <div className="space-y-4">
      {/* Header row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <button
            onClick={() => setEnabled(!enabled)}
            className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-bronze-500 focus:ring-offset-2 focus:ring-offset-bronze-950 ${
              enabled ? 'bg-bronze-600' : 'bg-bronze-800'
            }`}
            aria-label={enabled ? 'Disable EQ' : 'Enable EQ'}
          >
            <span
              className={`inline-block h-3.5 w-3.5 transform rounded-full bg-bronze-100 transition-transform ${
                enabled ? 'translate-x-4' : 'translate-x-0.5'
              }`}
            />
          </button>
          <span className="text-xs text-bronze-400">
            {enabled ? 'Active' : 'Bypassed'}
          </span>
        </div>

        <div className="flex items-center gap-2">
          {saving ? (
            <>
              <input
                type="text"
                value={presetName}
                onChange={(e) => setPresetName(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSave()}
                placeholder="Preset name"
                className="w-28 px-2 py-0.5 text-xs bg-bronze-950 border border-bronze-700 rounded text-bronze-200 placeholder-bronze-600 focus:outline-none focus:border-bronze-500"
                autoFocus
              />
              <button
                onClick={handleSave}
                className="text-xs text-bronze-400 hover:text-bronze-200 transition-colors"
              >
                Save
              </button>
              <button
                onClick={() => { setSaving(false); setPresetName('') }}
                className="text-xs text-bronze-600 hover:text-bronze-400 transition-colors"
              >
                Cancel
              </button>
            </>
          ) : (
            <button
              onClick={handleSave}
              className="text-xs text-bronze-500 hover:text-bronze-300 transition-colors"
            >
              Save preset
            </button>
          )}
          <button
            onClick={reset}
            className="text-xs text-bronze-600 hover:text-bronze-400 transition-colors"
          >
            Reset
          </button>
        </div>
      </div>

      {/* Preset chips */}
      <div className="flex flex-wrap gap-1.5">
        {builtInNames.map((name) => (
          <button
            key={name}
            onClick={() => setPreset(name)}
            className={`px-2.5 py-0.5 rounded-full text-xs transition-colors ${
              activePreset === name
                ? 'bg-bronze-600 text-white'
                : 'bg-bronze-900 border border-bronze-700 text-bronze-400 hover:border-bronze-500 hover:text-bronze-200'
            }`}
          >
            {name}
          </button>
        ))}
        {customNames.map((name) => (
          <div key={name} className="flex items-center gap-0.5">
            <button
              onClick={() => setPreset(name)}
              className={`px-2.5 py-0.5 rounded-full text-xs transition-colors ${
                activePreset === name
                  ? 'bg-copper-700 text-white'
                  : 'bg-bronze-900 border border-bronze-700 text-bronze-400 hover:border-bronze-500 hover:text-bronze-200'
              }`}
            >
              {name}
            </button>
            <button
              onClick={() => deleteCustomPreset(name)}
              className="text-bronze-700 hover:text-bronze-500 transition-colors text-xs leading-none"
              aria-label={`Delete preset ${name}`}
            >
              ×
            </button>
          </div>
        ))}
      </div>

      {/* Band sliders */}
      <div className="flex gap-1 items-end justify-between">
        {FREQUENCIES.map((freq, i) => (
          <div key={freq} className="flex flex-col items-center gap-1.5 flex-1">
            <span className="text-xs text-bronze-500 tabular-nums w-full text-center">
              {bands[i] !== undefined && bands[i] !== 0
                ? `${bands[i] > 0 ? '+' : ''}${bands[i]}`
                : '0'}
            </span>

            <div className="relative h-28 flex items-center justify-center">
              <input
                type="range"
                min={-12}
                max={12}
                step={0.5}
                value={bands[i] ?? 0}
                disabled={!enabled}
                onChange={(e) => setBand(i, Number.parseFloat(e.target.value))}
                className="h-24 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
                style={{
                  writingMode: 'vertical-lr',
                  direction: 'rtl',
                  WebkitAppearance: 'slider-vertical',
                  width: '28px',
                } as React.CSSProperties}
                aria-label={`${formatFreq(freq)} Hz gain`}
              />
            </div>

            <span className="text-xs text-bronze-500 tabular-nums">
              {formatFreq(freq)}
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}
