import { useState } from "react";
import type {
  CrossfeedConfig,
  ReplayGainConfig,
  CompressorConfig,
  VolumeConfig,
  ReplayGainMode,
} from "../../types/dsp";

interface DspControlsProps {
  crossfeed: CrossfeedConfig;
  replaygain: ReplayGainConfig;
  compressor: CompressorConfig;
  volume: VolumeConfig;
  onCrossfeedChange: (cfg: CrossfeedConfig) => void;
  onReplaygainChange: (cfg: ReplayGainConfig) => void;
  onCompressorChange: (cfg: CompressorConfig) => void;
  onVolumeChange: (cfg: VolumeConfig) => void;
}

function Section({
  title,
  enabled,
  onToggle,
  children,
}: {
  title: string;
  enabled: boolean;
  onToggle: (v: boolean) => void;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(true);

  return (
    <div className="bg-gray-800 rounded-lg overflow-hidden">
      <button
        onClick={() => setOpen(!open)}
        className="w-full flex items-center justify-between px-4 py-3 hover:bg-gray-750"
      >
        <div className="flex items-center gap-3">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => {
              e.stopPropagation();
              onToggle(e.target.checked);
            }}
            onClick={(e) => e.stopPropagation()}
            className="rounded border-gray-600 bg-gray-700 text-indigo-500 focus:ring-indigo-500 focus:ring-offset-0"
          />
          <span
            className={`text-sm font-medium ${enabled ? "text-white" : "text-gray-500"}`}
          >
            {title}
          </span>
        </div>
        <span className="text-gray-500 text-xs">{open ? "Hide" : "Show"}</span>
      </button>
      {open && <div className="px-4 pb-4 space-y-3">{children}</div>}
    </div>
  );
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  unit,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  unit: string;
  onChange: (v: number) => void;
}) {
  return (
    <div className="space-y-1">
      <div className="flex justify-between">
        <label className="text-xs text-gray-400">{label}</label>
        <span className="text-xs font-mono text-gray-400">
          {value.toFixed(step < 1 ? 1 : 0)} {unit}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-indigo-500"
      />
    </div>
  );
}

export default function DspControls({
  crossfeed,
  replaygain,
  compressor,
  volume,
  onCrossfeedChange,
  onReplaygainChange,
  onCompressorChange,
  onVolumeChange,
}: DspControlsProps) {
  return (
    <div className="space-y-3">
      {/* Crossfeed */}
      <Section
        title="Crossfeed"
        enabled={crossfeed.enabled}
        onToggle={(v) => onCrossfeedChange({ ...crossfeed, enabled: v })}
      >
        <p className="text-xs text-gray-500">
          Blends stereo channels for natural headphone listening.
        </p>
        <Slider
          label="Strength"
          value={Math.round(crossfeed.strength * 100)}
          min={0}
          max={100}
          step={1}
          unit="%"
          onChange={(v) =>
            onCrossfeedChange({ ...crossfeed, strength: v / 100 })
          }
        />
      </Section>

      {/* ReplayGain */}
      <Section
        title="ReplayGain"
        enabled={replaygain.enabled}
        onToggle={(v) => onReplaygainChange({ ...replaygain, enabled: v })}
      >
        <p className="text-xs text-gray-500">
          Normalizes loudness across tracks using embedded gain tags.
        </p>
        <div className="space-y-1">
          <label className="text-xs text-gray-400">Mode</label>
          <div className="flex gap-2">
            {(["Track", "Album"] as ReplayGainMode[]).map((mode) => (
              <button
                key={mode}
                onClick={() =>
                  onReplaygainChange({ ...replaygain, mode })
                }
                className={`px-3 py-1 text-xs rounded transition-colors ${
                  replaygain.mode === mode
                    ? "bg-indigo-600 text-white"
                    : "bg-gray-700 text-gray-400 hover:text-white"
                }`}
              >
                {mode}
              </button>
            ))}
          </div>
        </div>
        <Slider
          label="Preamp"
          value={replaygain.preamp_db}
          min={-12}
          max={12}
          step={0.5}
          unit="dB"
          onChange={(v) =>
            onReplaygainChange({ ...replaygain, preamp_db: v })
          }
        />
        <label className="flex items-center gap-2 text-xs text-gray-400">
          <input
            type="checkbox"
            checked={replaygain.prevent_clipping}
            onChange={(e) =>
              onReplaygainChange({
                ...replaygain,
                prevent_clipping: e.target.checked,
              })
            }
            className="rounded border-gray-600 bg-gray-700 text-indigo-500 focus:ring-indigo-500 focus:ring-offset-0"
          />
          Prevent clipping
        </label>
        <label className="flex items-center gap-2 text-xs text-gray-400">
          <input
            type="checkbox"
            checked={replaygain.fallback_to_track}
            onChange={(e) =>
              onReplaygainChange({
                ...replaygain,
                fallback_to_track: e.target.checked,
              })
            }
            className="rounded border-gray-600 bg-gray-700 text-indigo-500 focus:ring-indigo-500 focus:ring-offset-0"
          />
          Fall back to track gain when album gain unavailable
        </label>
      </Section>

      {/* Compressor */}
      <Section
        title="Compressor"
        enabled={compressor.enabled}
        onToggle={(v) => onCompressorChange({ ...compressor, enabled: v })}
      >
        <p className="text-xs text-gray-500">
          Reduces dynamic range for late-night listening.
        </p>
        <Slider
          label="Threshold"
          value={compressor.threshold_db}
          min={-30}
          max={0}
          step={0.5}
          unit="dB"
          onChange={(v) =>
            onCompressorChange({ ...compressor, threshold_db: v })
          }
        />
        <Slider
          label="Ratio"
          value={compressor.ratio}
          min={1}
          max={20}
          step={0.5}
          unit=":1"
          onChange={(v) =>
            onCompressorChange({ ...compressor, ratio: v })
          }
        />
        <Slider
          label="Attack"
          value={compressor.attack_ms}
          min={0.1}
          max={100}
          step={0.1}
          unit="ms"
          onChange={(v) =>
            onCompressorChange({ ...compressor, attack_ms: v })
          }
        />
        <Slider
          label="Release"
          value={compressor.release_ms}
          min={10}
          max={1000}
          step={10}
          unit="ms"
          onChange={(v) =>
            onCompressorChange({ ...compressor, release_ms: v })
          }
        />
        <Slider
          label="Limiter Ceiling"
          value={compressor.limiter_ceiling_db}
          min={-6}
          max={0}
          step={0.1}
          unit="dB"
          onChange={(v) =>
            onCompressorChange({ ...compressor, limiter_ceiling_db: v })
          }
        />
      </Section>

      {/* Volume */}
      <Section
        title="Volume"
        enabled={true}
        onToggle={() => {}}
      >
        <Slider
          label="Level"
          value={volume.level_db}
          min={-60}
          max={0}
          step={0.5}
          unit="dB"
          onChange={(v) => onVolumeChange({ ...volume, level_db: v })}
        />
        <label className="flex items-center gap-2 text-xs text-gray-400">
          <input
            type="checkbox"
            checked={volume.dither}
            onChange={(e) =>
              onVolumeChange({ ...volume, dither: e.target.checked })
            }
            className="rounded border-gray-600 bg-gray-700 text-indigo-500 focus:ring-indigo-500 focus:ring-offset-0"
          />
          TPDF dither
        </label>
      </Section>
    </div>
  );
}
