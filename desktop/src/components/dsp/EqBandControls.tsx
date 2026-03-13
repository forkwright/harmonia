import type { EqBand, FilterType } from "../../types/dsp";

const FILTER_TYPES: FilterType[] = [
  "Peaking",
  "LowShelf",
  "HighShelf",
  "LowPass",
  "HighPass",
  "Notch",
  "AllPass",
];

const PRESET_FREQS = [
  31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000,
];

interface EqBandControlsProps {
  band: EqBand;
  index: number;
  onChange: (index: number, updates: Partial<EqBand>) => void;
  onRemove: (index: number) => void;
}

function formatFreq(hz: number): string {
  return hz >= 1000 ? `${(hz / 1000).toFixed(hz % 1000 === 0 ? 0 : 1)}k` : `${hz}`;
}

export default function EqBandControls({
  band,
  index,
  onChange,
  onRemove,
}: EqBandControlsProps) {
  return (
    <div className="bg-gray-800 rounded-lg p-4 space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-gray-300">
          Band {index + 1} — {formatFreq(band.frequency)} Hz
        </span>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-1.5 text-xs text-gray-400">
            <input
              type="checkbox"
              checked={band.enabled}
              onChange={(e) => onChange(index, { enabled: e.target.checked })}
              className="rounded border-gray-600 bg-gray-700 text-indigo-500 focus:ring-indigo-500 focus:ring-offset-0"
            />
            On
          </label>
          <button
            onClick={() => onRemove(index)}
            className="text-gray-500 hover:text-red-400 text-xs px-1"
            title="Remove band"
          >
            Remove
          </button>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3">
        {/* Frequency */}
        <div className="space-y-1">
          <label className="text-xs text-gray-400">Frequency</label>
          <div className="flex gap-1">
            <select
              value={PRESET_FREQS.includes(band.frequency) ? band.frequency : "custom"}
              onChange={(e) => {
                const val = e.target.value;
                if (val !== "custom") {
                  onChange(index, { frequency: Number(val) });
                }
              }}
              className="flex-1 bg-gray-700 border border-gray-600 rounded px-2 py-1 text-sm text-white"
            >
              {PRESET_FREQS.map((f) => (
                <option key={f} value={f}>
                  {formatFreq(f)} Hz
                </option>
              ))}
              {!PRESET_FREQS.includes(band.frequency) && (
                <option value="custom">{band.frequency} Hz</option>
              )}
            </select>
            <input
              type="number"
              min={20}
              max={20000}
              value={band.frequency}
              onChange={(e) =>
                onChange(index, {
                  frequency: Math.max(20, Math.min(20000, Number(e.target.value))),
                })
              }
              className="w-20 bg-gray-700 border border-gray-600 rounded px-2 py-1 text-sm text-white"
            />
          </div>
        </div>

        {/* Filter Type */}
        <div className="space-y-1">
          <label className="text-xs text-gray-400">Type</label>
          <select
            value={band.filter_type}
            onChange={(e) =>
              onChange(index, { filter_type: e.target.value as FilterType })
            }
            className="w-full bg-gray-700 border border-gray-600 rounded px-2 py-1 text-sm text-white"
          >
            {FILTER_TYPES.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Gain */}
      <div className="space-y-1">
        <div className="flex justify-between">
          <label className="text-xs text-gray-400">Gain</label>
          <span
            className={`text-xs font-mono ${
              band.gain_db > 0
                ? "text-amber-400"
                : band.gain_db < 0
                  ? "text-blue-400"
                  : "text-gray-400"
            }`}
          >
            {band.gain_db > 0 ? "+" : ""}
            {band.gain_db.toFixed(1)} dB
          </span>
        </div>
        <input
          type="range"
          min={-15}
          max={15}
          step={0.1}
          value={band.gain_db}
          onChange={(e) => onChange(index, { gain_db: Number(e.target.value) })}
          className="w-full accent-indigo-500"
        />
      </div>

      {/* Q */}
      <div className="space-y-1">
        <div className="flex justify-between">
          <label className="text-xs text-gray-400">Q</label>
          <span className="text-xs font-mono text-gray-400">
            {band.q.toFixed(1)}
          </span>
        </div>
        <input
          type="range"
          min={0.1}
          max={10}
          step={0.1}
          value={band.q}
          onChange={(e) => onChange(index, { q: Number(e.target.value) })}
          className="w-full accent-indigo-500"
        />
      </div>
    </div>
  );
}
