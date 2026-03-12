import type { EqPreset } from "../../types/dsp";

interface EqToolbarProps {
  enabled: boolean;
  preampDb: number;
  presets: EqPreset[];
  onToggle: (enabled: boolean) => void;
  onPreampChange: (db: number) => void;
  onPresetLoad: (name: string) => void;
  onAddBand: () => void;
  onReset: () => void;
}

export default function EqToolbar({
  enabled,
  preampDb,
  presets,
  onToggle,
  onPreampChange,
  onPresetLoad,
  onAddBand,
  onReset,
}: EqToolbarProps) {
  return (
    <div className="flex items-center gap-4 flex-wrap">
      {/* Enable toggle */}
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => onToggle(e.target.checked)}
          className="rounded border-gray-600 bg-gray-700 text-indigo-500 focus:ring-indigo-500 focus:ring-offset-0"
        />
        <span className={enabled ? "text-white" : "text-gray-500"}>EQ</span>
      </label>

      {/* Preamp */}
      <div className="flex items-center gap-2">
        <label className="text-xs text-gray-400">Preamp</label>
        <input
          type="range"
          min={-12}
          max={12}
          step={0.5}
          value={preampDb}
          onChange={(e) => onPreampChange(Number(e.target.value))}
          className="w-24 accent-indigo-500"
        />
        <span className="text-xs font-mono text-gray-400 w-14 text-right">
          {preampDb > 0 ? "+" : ""}
          {preampDb.toFixed(1)} dB
        </span>
      </div>

      {/* Preset selector */}
      <select
        onChange={(e) => {
          if (e.target.value) {
            onPresetLoad(e.target.value);
            e.target.value = "";
          }
        }}
        defaultValue=""
        className="bg-gray-700 border border-gray-600 rounded px-2 py-1 text-sm text-white"
      >
        <option value="" disabled>
          Presets
        </option>
        {presets.map((p) => (
          <option key={p.name} value={p.name}>
            {p.name}
          </option>
        ))}
      </select>

      <div className="flex-1" />

      {/* Actions */}
      <button
        onClick={onAddBand}
        className="px-3 py-1 bg-gray-700 hover:bg-gray-600 text-sm text-white rounded transition-colors"
      >
        Add Band
      </button>
      <button
        onClick={onReset}
        className="px-3 py-1 bg-gray-700 hover:bg-gray-600 text-sm text-gray-400 hover:text-white rounded transition-colors"
      >
        Reset
      </button>
    </div>
  );
}
