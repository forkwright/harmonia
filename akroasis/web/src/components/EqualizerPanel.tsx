// 10-band parametric EQ panel with preset chips, enable toggle, band sliders, and dynamics compressor
import { useState, useMemo } from 'react'
import { useEqStore, BUILT_IN_PRESETS } from '../stores/eqStore'
import { useCompressorStore, COMPRESSOR_PRESETS } from '../stores/compressorStore'
import { HEADPHONE_PROFILES } from '../data/headphoneProfiles'
import { searchProfiles, groupByManufacturer } from '../audio/autoEqConverter'

const FREQUENCIES = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]

function formatFreq(hz: number): string {
  return hz >= 1000 ? `${hz / 1000}k` : `${hz}`
}

export function EqualizerPanel() {
  const {
    enabled,
    bands,
    activePreset,
    activeHeadphoneProfile,
    customPresets,
    setBand,
    setPreset,
    saveCustomPreset,
    deleteCustomPreset,
    setEnabled,
    applyHeadphoneProfile,
    clearHeadphoneProfile,
    reset,
  } = useEqStore()

  const [saving, setSaving] = useState(false)
  const [presetName, setPresetName] = useState('')
  const [hpExpanded, setHpExpanded] = useState(false)
  const [hpSearch, setHpSearch] = useState('')

  const filteredProfiles = useMemo(() => searchProfiles(HEADPHONE_PROFILES, hpSearch), [hpSearch])
  const groupedProfiles = useMemo(() => groupByManufacturer(filteredProfiles), [filteredProfiles])

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

      {/* Headphone EQ */}
      <div className="border border-bronze-800 rounded-lg overflow-hidden">
        <button
          onClick={() => setHpExpanded(!hpExpanded)}
          className="w-full flex items-center justify-between px-3 py-2 text-sm text-bronze-300 hover:bg-bronze-900/50 transition-colors"
        >
          <span className="flex items-center gap-2">
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
            </svg>
            Headphone EQ
          </span>
          <span className="flex items-center gap-2">
            {activeHeadphoneProfile && (
              <span className="text-xs text-bronze-500 bg-bronze-800 px-2 py-0.5 rounded-full flex items-center gap-1">
                {activeHeadphoneProfile}
                <button
                  onClick={(e) => { e.stopPropagation(); clearHeadphoneProfile() }}
                  className="text-bronze-600 hover:text-bronze-400 ml-0.5"
                  aria-label="Clear headphone profile"
                >
                  ×
                </button>
              </span>
            )}
            {!activeHeadphoneProfile && (
              <span className="text-xs text-bronze-600">None</span>
            )}
            <svg
              className={`w-4 h-4 text-bronze-600 transition-transform ${hpExpanded ? 'rotate-180' : ''}`}
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
            </svg>
          </span>
        </button>

        {hpExpanded && (
          <div className="border-t border-bronze-800 px-3 py-2 space-y-2">
            <input
              type="text"
              value={hpSearch}
              onChange={(e) => setHpSearch(e.target.value)}
              placeholder="Search headphones..."
              className="w-full px-2 py-1 text-xs bg-bronze-950 border border-bronze-700 rounded text-bronze-200 placeholder-bronze-600 focus:outline-none focus:border-bronze-500"
            />
            <div className="max-h-48 overflow-y-auto space-y-2">
              {Array.from(groupedProfiles.entries()).map(([manufacturer, profiles]) => (
                <div key={manufacturer}>
                  <p className="text-[10px] font-semibold text-bronze-500 uppercase tracking-wider mb-1">
                    {manufacturer}
                  </p>
                  <div className="flex flex-wrap gap-1">
                    {profiles.map((profile) => {
                      const fullName = `${profile.manufacturer} ${profile.model}`
                      return (
                        <button
                          key={fullName}
                          onClick={() => { applyHeadphoneProfile(profile); setHpExpanded(false); setHpSearch('') }}
                          className={`px-2 py-0.5 rounded text-xs transition-colors ${
                            activeHeadphoneProfile === fullName
                              ? 'bg-bronze-600 text-white'
                              : 'bg-bronze-900 text-bronze-400 hover:bg-bronze-800 hover:text-bronze-200'
                          }`}
                        >
                          {profile.model}
                        </button>
                      )
                    })}
                  </div>
                </div>
              ))}
              {filteredProfiles.length === 0 && (
                <p className="text-xs text-bronze-600 py-2 text-center">No matching headphones</p>
              )}
            </div>
          </div>
        )}
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

      <DynamicsSection />
    </div>
  )
}

function DynamicsSection() {
  const {
    enabled, activePreset, threshold, knee, ratio, attack, release,
    setEnabled, setPreset, setParam, reset,
  } = useCompressorStore()

  const [expanded, setExpanded] = useState(false)
  const presetNames = Object.keys(COMPRESSOR_PRESETS)

  return (
    <div className="border-t border-bronze-800 pt-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <button
            onClick={() => setEnabled(!enabled)}
            className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-bronze-500 focus:ring-offset-2 focus:ring-offset-bronze-950 ${
              enabled ? 'bg-bronze-600' : 'bg-bronze-800'
            }`}
            aria-label={enabled ? 'Disable compressor' : 'Enable compressor'}
          >
            <span
              className={`inline-block h-3.5 w-3.5 transform rounded-full bg-bronze-100 transition-transform ${
                enabled ? 'translate-x-4' : 'translate-x-0.5'
              }`}
            />
          </button>
          <span className="text-sm text-bronze-300">Dynamics</span>
          <span className="text-xs text-bronze-500">
            {enabled ? (activePreset ?? 'Custom') : 'Off'}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setExpanded(!expanded)}
            className="text-xs text-bronze-500 hover:text-bronze-300 transition-colors"
          >
            {expanded ? 'Collapse' : 'Expand'}
          </button>
          <button
            onClick={reset}
            className="text-xs text-bronze-600 hover:text-bronze-400 transition-colors"
          >
            Reset
          </button>
        </div>
      </div>

      <div className="flex flex-wrap gap-1.5">
        {presetNames.map((name) => (
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
      </div>

      {expanded && (
        <div className="space-y-2 pt-1">
          <DynamicsSlider label="Threshold" value={threshold} min={-100} max={0} step={1} unit="dB" onChange={(v) => setParam('threshold', v)} />
          <DynamicsSlider label="Knee" value={knee} min={0} max={40} step={1} unit="dB" onChange={(v) => setParam('knee', v)} />
          <DynamicsSlider label="Ratio" value={ratio} min={1} max={20} step={0.5} unit=":1" onChange={(v) => setParam('ratio', v)} />
          <DynamicsSlider label="Attack" value={attack} min={0} max={1} step={0.001} unit="s" onChange={(v) => setParam('attack', v)} />
          <DynamicsSlider label="Release" value={release} min={0} max={1} step={0.01} unit="s" onChange={(v) => setParam('release', v)} />
        </div>
      )}
    </div>
  )
}

function DynamicsSlider({
  label, value, min, max, step, unit, onChange,
}: {
  label: string; value: number; min: number; max: number; step: number; unit: string; onChange: (v: number) => void
}) {
  return (
    <div className="flex items-center gap-3">
      <span className="text-xs text-bronze-500 w-16 text-right">{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number.parseFloat(e.target.value))}
        className="flex-1 h-1.5 cursor-pointer"
        aria-label={`${label} ${value}${unit}`}
      />
      <span className="text-xs text-bronze-400 tabular-nums w-14 text-right">
        {step < 0.1 ? value.toFixed(3) : step < 1 ? value.toFixed(1) : value}{unit}
      </span>
    </div>
  )
}
